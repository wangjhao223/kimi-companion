//! CodexService：把 Windows 端 Codex 的 rollout 会话文件增量导入 SQLite（codex_events）。
//!
//! - 数据源：`%USERPROFILE%\.codex\sessions\**\rollout-*.jsonl`
//!   以及 `archived_sessions`（线程被删除后文件移入此处，用量仍是真实发生的）；
//! - 每个文件按 meta["codex_off:<path>"] 字节偏移增量读，尾部残缺行留到下轮；
//! - offset 超过文件大小（文件被重建）回 0 重读，靠 UNIQUE + INSERT OR IGNORE 去重；
//! - 会话文件在 sessions/ 与 archived_sessions/ 之间移动会形成新路径从而全量重读，
//!   去重键 (ts, session_id, model, input, output) 保证不重复计数；
//! - rollout 行格式：顶层 `type` 为 `event_msg` 的 token_count 事件，
//!   用量在 `payload.info.last_token_usage`；模型名不在 session_meta 里，
//!   而在每个回合的 `turn_context` 行（`payload.model`，回合间可切换），
//!   最近看到的模型名按文件持久化在 meta["codex_model:<path>"] 供增量读取恢复；
//! - 解析规则变更时递增 PARSER_VERSION，启动迁移会清空偏移全量重读；
//! - 后台线程每 15 秒一轮，首轮全量 backfill；所有错误只进 CodexStatus，线程不 panic。

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::database::{dao, Database};
use crate::models::CodexStatus;

const SYNC_INTERVAL: Duration = Duration::from_secs(15);
const OFFSET_KEY_PREFIX: &str = "codex_off:";
const MODEL_KEY_PREFIX: &str = "codex_model:";
/// rollout 解析规则版本：格式适配变更时 +1，触发一次全量重读。
const PARSER_VERSION: &str = "2";

/// 内存中的 Codex 同步状态，供 get_codex_status 命令查询。
pub type SharedCodexStatus = Arc<Mutex<CodexStatus>>;

pub fn new_shared_status() -> SharedCodexStatus {
    Arc::new(Mutex::new(CodexStatus::default()))
}

/// 启动后台同步线程。detached，不 join，随进程退出结束。
pub fn spawn_sync_thread(db: Database, status: SharedCodexStatus) {
    std::thread::Builder::new()
        .name("codex-sync".to_string())
        .spawn(move || {
            migrate_offsets(&db);
            loop {
                sync_round(&db, &status);
                std::thread::sleep(SYNC_INTERVAL);
            }
        })
        .expect("无法启动 codex 同步线程");
}

/// 解析器版本迁移：版本不符时清空所有文件偏移，下一轮即全量重读
/// （UNIQUE + INSERT OR IGNORE 保证重读不重复计数，无需清表）。
fn migrate_offsets(db: &Database) {
    let current = db
        .with_conn(|c| dao::get_meta(c, "codex_parser_v"))
        .ok()
        .flatten();
    if current.as_deref() == Some(PARSER_VERSION) {
        return;
    }
    let _ = db.with_conn(|c| dao::delete_meta_prefix(c, OFFSET_KEY_PREFIX));
    let _ = db.with_conn(|c| dao::set_meta(c, "codex_parser_v", PARSER_VERSION));
}

/// 一轮同步：扫 sessions/ 与 archived_sessions/ 下所有 rollout 文件，逐个增量导入。
pub fn sync_round(db: &Database, status: &SharedCodexStatus) {
    let root = codex_root().filter(|p| p.is_dir());
    let mut error: Option<String> = None;

    if let Some(root) = &root {
        let mut files = Vec::new();
        collect_jsonl(&root.join("sessions"), &mut files);
        collect_jsonl(&root.join("archived_sessions"), &mut files);
        // 单个文件出错不拖垮整轮：其余文件照常导入，错误只展示最后一个
        for f in &files {
            if let Err(e) = sync_file(db, f) {
                error = Some(format!("{}: {e}", f.display()));
            }
        }
    }

    let mut guard = match status.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    guard.dir_found = root.is_some();
    guard.dir = root.as_ref().map(|p| p.display().to_string());
    if root.is_some() {
        guard.last_sync_at_ms = Some(now_ms());
    }
    guard.last_error = if root.is_some() {
        error
    } else {
        Some("未找到 Codex 数据目录（Codex 未安装或尚未使用过）".to_string())
    };
    // 总记录数本地可查，同步失败也照常刷新
    if let Ok(total) = db.with_conn(|c| dao::count_codex_events(c)) {
        guard.total_records = total;
    }
}

/// Codex 数据目录：Windows 上固定在用户配置目录下。
fn codex_root() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|p| PathBuf::from(p).join(".codex"))
}

/// 递归收集 dir 下所有 .jsonl 文件。目录不存在时静默返回空。
fn collect_jsonl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl(&path, out);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            out.push(path);
        }
    }
}

/// 从文件第一行（session_meta 固定在第一行）恢复会话 id。
/// 用于增量读取（offset > 0）时 session_meta 已经消费过的场景。
/// 注意：session_meta 不含模型名，模型由 meta["codex_model:<path>"] 恢复。
fn read_first_meta(file: &mut std::fs::File) -> Result<Option<FileMeta>, String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    // meta 行含 base_instructions，可能有几十 KB；读到换行即止，上限 4MB 保底
    while buf.len() <= 4 * 1024 * 1024 {
        let n = file
            .read(&mut chunk)
            .map_err(|e| format!("读取 session_meta 失败: {e}"))?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if chunk[..n].contains(&b'\n') {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf);
    Ok(parse_meta_line(text.lines().next().unwrap_or("")))
}

/// 同步单个 rollout 文件的增量，返回本轮新插入条数。
fn sync_file(db: &Database, path: &Path) -> Result<usize, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开失败: {e}"))?;
    let size = file
        .metadata()
        .map_err(|e| format!("读取元数据失败: {e}"))?
        .len();

    let offset_key = format!("{OFFSET_KEY_PREFIX}{}", path.display());
    let model_key = format!("{MODEL_KEY_PREFIX}{}", path.display());
    let mut offset: u64 = db
        .with_conn(|c| dao::get_meta(c, &offset_key))?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if offset > size {
        // 文件被截断或重建，从头重读（INSERT OR IGNORE 保证不重复）
        offset = 0;
    }
    if offset == size {
        return Ok(0);
    }

    // 增量路径下 session_meta 已消费过：从第一行恢复会话 id，从 meta 表恢复模型名
    let mut meta = if offset > 0 {
        let mut m = read_first_meta(&mut file)?.unwrap_or_default();
        if m.model.is_empty() {
            m.model = db
                .with_conn(|c| dao::get_meta(c, &model_key))?
                .unwrap_or_default();
        }
        m
    } else {
        FileMeta::default()
    };

    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("seek 失败: {e}"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("读取失败: {e}"))?;

    // 只消费到最后一个换行符：尾部可能是 Codex 正在写入的残缺行，留到下轮
    let complete_len = match buf.iter().rposition(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => return Ok(0),
    };

    let text = String::from_utf8_lossy(&buf[..complete_len]);
    let fallback_id = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rows: Vec<dao::UsageEventRow> = Vec::new();
    for line in text.lines() {
        if has_type(line, "session_meta") {
            if let Some(m) = parse_meta_line(line) {
                if !m.session_id.is_empty() {
                    meta.session_id = m.session_id;
                }
                if !m.model.is_empty() {
                    meta.model = m.model;
                }
            }
        } else if has_type(line, "turn_context") {
            // 模型名在 turn_context 里，回合间可能切换：总是取最新值
            if let Some(model) = parse_turn_model(line) {
                meta.model = model;
            }
        } else if has_type(line, "token_count") {
            if let Some(row) = parse_token_line(line, &meta, &fallback_id) {
                rows.push(row);
            }
        }
    }

    let inserted = db.with_conn_mut(|c| dao::insert_codex_events(c, &rows))?;
    // 持久化最新模型名，供增量读取恢复
    if !meta.model.is_empty() {
        db.with_conn(|c| dao::set_meta(c, &model_key, &meta.model))?;
    }
    let new_offset = offset + complete_len as u64;
    db.with_conn(|c| dao::set_meta(c, &offset_key, &new_offset.to_string()))?;
    Ok(inserted)
}

/// 会话级元信息：会话 id + 当前回合模型名（随 turn_context 滚动更新）。
#[derive(Default)]
struct FileMeta {
    session_id: String,
    model: String,
}

/// 行类型预筛：同时兼容紧凑（"type":"x"）与带空格（"type": "x"）两种序列化。
/// 只是快速过滤，真正的字段校验在 serde 解析后做。
fn has_type(line: &str, t: &str) -> bool {
    line.contains(&format!("\"type\":\"{t}\"")) || line.contains(&format!("\"type\": \"{t}\""))
}

/// 宽松解析 session_meta 行：非 session_meta 或非法 JSON 返回 None。
fn parse_meta_line(line: &str) -> Option<FileMeta> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "session_meta" {
        return None;
    }
    let p = v.get("payload")?;
    Some(FileMeta {
        session_id: p
            .get("session_id")
            .or_else(|| p.get("id"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        model: p
            .get("model")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// 宽松解析 turn_context 行：取 payload.model。
fn parse_turn_model(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "turn_context" {
        return None;
    }
    v.get("payload")?
        .get("model")?
        .as_str()
        .map(String::from)
}

/// 宽松解析 token_count 行：顶层 type 是 event_msg，用量在
/// payload.info.last_token_usage（total_token_usage 是累计值，按增量入库
/// 才能正确落到每天）。缺时间戳或用量体则跳过整行。
///
/// 字段映射（与 kimi 侧四列对齐）：cached 是 input 的子集，reasoning 是 output
/// 的子集，均不重复计数：
/// - input          = input_tokens - cached_input_tokens（未命中缓存的输入）
/// - cache_read     = cached_input_tokens
/// - cache_creation = cache_write_input_tokens
/// - output         = output_tokens（含 reasoning）
fn parse_token_line(line: &str, meta: &FileMeta, fallback_id: &str) -> Option<dao::UsageEventRow> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = v.get("payload")?;
    if payload.get("type")?.as_str()? != "token_count" {
        return None;
    }
    let ts = rfc3339_utc_to_ms(v.get("timestamp")?.as_str()?)?;
    let u = payload.get("info")?.get("last_token_usage")?;
    let n = |k: &str| u.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
    let cached = n("cached_input_tokens");
    Some(dao::UsageEventRow {
        ts,
        session_id: if meta.session_id.is_empty() {
            fallback_id.to_string()
        } else {
            meta.session_id.clone()
        },
        agent: String::new(), // codex_events 无 agent 列，仅占位
        model: meta.model.clone(),
        input_tokens: (n("input_tokens") - cached).max(0),
        output_tokens: n("output_tokens"),
        cache_read: cached,
        cache_creation: n("cache_write_input_tokens"),
    })
}

/// 解析 rollout 行时间戳（固定 `YYYY-MM-DDTHH:MM:SS[.frac]Z`，UTC）为毫秒时间戳。
/// 格式不符一律返回 None（该行跳过），不做完整 RFC3339 兼容。
fn rfc3339_utc_to_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let min: i64 = s.get(14..16)?.parse().ok()?;
    let sec: i64 = s.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let rest = &s[19..];
    let (frac_ms, z) = match rest.strip_prefix('.') {
        Some(f) => {
            let digits: String = f.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                return None;
            }
            // 归一到毫秒：多于 3 位截断，不足补零
            let mut ms: i64 = digits.parse().ok()?;
            match digits.len() {
                l if l > 3 => ms /= 10i64.pow((l - 3) as u32),
                l => ms *= 10i64.pow((3 - l) as u32),
            }
            (ms, &f[digits.len()..])
        }
        None => (0, rest),
    };
    if z != "Z" {
        return None;
    }

    // days-from-civil（Howard Hinnant 算法）：公历日期 → 纪元天数
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(((days * 24 + hour) * 60 + min) * 60_000 + sec * 1000 + frac_ms)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
