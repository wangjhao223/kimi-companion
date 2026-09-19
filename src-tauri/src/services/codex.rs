//! CodexService：把两个平台的 Codex rollout 会话文件增量导入 SQLite。
//!
//! 两个数据源共用同一套 rollout 解析逻辑，按「源」路由（见 Source）：
//! - Windows 侧：`%USERPROFILE%\.codex\sessions\**\rollout-*.jsonl`
//!   以及 `archived_sessions`（线程被删除后文件移入此处，用量仍是真实发生的），
//!   入 codex_events；偏移存 meta["codex_off:<path>"]，模型名存 meta["codex_model:<path>"]；
//! - WSL 侧：枚举发行版（wsl::list_distros，结果缓存 60 秒）拼 UNC 根
//!   `\\wsl$\<distro><按 $HOME 解析的家目录>\.codex`（兜底 `\\wsl.localhost\...`，
//!   每个发行版只认第一个能读到目录的前缀），目录不存在不算错误（dir_found=false），
//!   入 wsl_codex_events；偏移存 meta["wsl_codex_off:<path>"]；
//! - 每个文件按字节偏移增量读，尾部残缺行留到下轮；
//! - offset 超过文件大小（文件被重建）回 0 重读，靠 UNIQUE + INSERT OR IGNORE 去重；
//! - 会话文件在 sessions/ 与 archived_sessions/ 之间移动会形成新路径从而全量重读，
//!   去重键 (ts, session_id, model, input, output) 保证不重复计数；
//! - rollout 行格式：顶层 `type` 为 `event_msg` 的 token_count 事件，
//!   用量在 `payload.info.last_token_usage`；模型名不在 session_meta 里，
//!   而在每个回合的 `turn_context` 行（`payload.model`，回合间可切换），
//!   最近看到的模型名按文件持久化在 meta["<源模型前缀>:<path>"] 供增量读取恢复；
//! - 解析规则变更时递增 PARSER_VERSION，启动迁移清空两个源的偏移前缀全量重读；
//! - 后台线程每 15 秒一轮，首轮全量 backfill；Windows 侧与 WSL 侧各自独立扫描、
//!   独立上报 CodexStatus，单文件出错不拖垮整轮，线程不 panic。

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::database::{dao, Database};
use crate::models::CodexStatus;
use crate::services::wsl;

const SYNC_INTERVAL: Duration = Duration::from_secs(15);
/// rollout 解析规则版本：格式适配变更时 +1，触发两个源各一次全量重读。
/// 版本键 meta["codex_parser_v"] 两源共享（解析器是同一个）。
const PARSER_VERSION: &str = "2";

/// 一个数据源的静态描述：落库目标与 meta 键前缀按源隔离，解析逻辑共用。
struct Source {
    /// 文件偏移 meta 键前缀（Windows 沿用 codex_off: 兼容存量）
    offset_prefix: &'static str,
    /// 最近模型名 meta 键前缀（增量读取时恢复）
    model_prefix: &'static str,
    /// 批量插入目标表
    insert: fn(&mut rusqlite::Connection, &[dao::UsageEventRow]) -> Result<usize, String>,
    /// 目标表总记录数（状态快照用）
    count: fn(&rusqlite::Connection) -> Result<i64, String>,
}

/// Windows 侧：%USERPROFILE%\.codex → codex_events。
const WINDOWS_SOURCE: Source = Source {
    offset_prefix: "codex_off:",
    model_prefix: "codex_model:",
    insert: dao::insert_codex_events,
    count: dao::count_codex_events,
};

/// WSL 侧：\\wsl$\<distro><按 $HOME 解析的家目录>\.codex → wsl_codex_events。
const WSL_SOURCE: Source = Source {
    offset_prefix: "wsl_codex_off:",
    model_prefix: "wsl_codex_model:",
    insert: dao::insert_wsl_codex_events,
    count: dao::count_wsl_codex_events,
};

/// 内存中的 Codex 同步状态（Windows 侧），供 get_codex_status 命令查询。
pub type SharedCodexStatus = Arc<Mutex<CodexStatus>>;

pub fn new_shared_status() -> SharedCodexStatus {
    Arc::new(Mutex::new(CodexStatus::default()))
}

/// 内存中的 Codex 同步状态（WSL 侧），供 get_wsl_codex_status 命令查询。
/// 与 Windows 侧同为 Arc<Mutex<CodexStatus>>，Tauri manage 按类型区分状态，
/// 需用 newtype 包装避免撞型。
#[derive(Clone)]
pub struct SharedWslCodexStatus(pub Arc<Mutex<CodexStatus>>);

pub fn new_shared_wsl_status() -> SharedWslCodexStatus {
    SharedWslCodexStatus(Arc::new(Mutex::new(CodexStatus::default())))
}

/// 启动后台同步线程。detached，不 join，随进程退出结束。
pub fn spawn_sync_thread(
    db: Database,
    win_status: SharedCodexStatus,
    wsl_status: SharedWslCodexStatus,
) {
    std::thread::Builder::new()
        .name("codex-sync".to_string())
        .spawn(move || {
            migrate_offsets(&db);
            loop {
                sync_round(&db, &win_status, &wsl_status);
                std::thread::sleep(SYNC_INTERVAL);
            }
        })
        .expect("无法启动 codex 同步线程");
}

/// 解析器版本迁移：版本不符时清空两个源的所有文件偏移，下一轮即全量重读
/// （UNIQUE + INSERT OR IGNORE 保证重读不重复计数，无需清表）。
fn migrate_offsets(db: &Database) {
    let current = db
        .with_conn(|c| dao::get_meta(c, "codex_parser_v"))
        .ok()
        .flatten();
    if current.as_deref() == Some(PARSER_VERSION) {
        return;
    }
    let _ = db.with_conn(|c| dao::delete_meta_prefix(c, WINDOWS_SOURCE.offset_prefix));
    let _ = db.with_conn(|c| dao::delete_meta_prefix(c, WSL_SOURCE.offset_prefix));
    let _ = db.with_conn(|c| dao::set_meta(c, "codex_parser_v", PARSER_VERSION));
}

/// 一轮同步：Windows 侧与 WSL 侧各自独立扫描、独立上报状态，互不拖垮。
pub fn sync_round(
    db: &Database,
    win_status: &SharedCodexStatus,
    wsl_status: &SharedWslCodexStatus,
) {
    sync_windows(db, win_status);
    sync_wsl(db, wsl_status);
}

/// Windows 侧一轮：扫 %USERPROFILE%\.codex 下所有 rollout 文件，逐个增量导入。
fn sync_windows(db: &Database, status: &SharedCodexStatus) {
    let root = codex_root().filter(|p| p.is_dir());
    let error = root
        .as_ref()
        .and_then(|root| sync_root(db, root, &WINDOWS_SOURCE));

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
    if let Ok(total) = db.with_conn(|c| (WINDOWS_SOURCE.count)(c)) {
        guard.total_records = total;
    }
}

/// WSL 侧一轮：枚举发行版（60 秒缓存），每个发行版拼 UNC 根逐个增量导入。
/// 目录不存在不算错误（没装 codex 是常态）；wsl.exe 枚举失败才进 last_error。
fn sync_wsl(db: &Database, status: &SharedWslCodexStatus) {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut enum_error: Option<String> = None;
    match cached_distros() {
        Ok(distros) => {
            for distro in &distros {
                if let Some(root) = wsl_codex_root(distro) {
                    roots.push(root);
                }
            }
        }
        Err(e) => enum_error = Some(e),
    }

    // 单文件出错不拖垮整轮：其余文件照常导入，错误只展示最后一个
    let mut scan_error: Option<String> = None;
    for root in &roots {
        if let Some(e) = sync_root(db, root, &WSL_SOURCE) {
            scan_error = Some(e);
        }
    }

    let found = !roots.is_empty();
    let mut guard = match status.0.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    guard.dir_found = found;
    guard.dir = if found {
        Some(
            roots
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    } else {
        None
    };
    if found {
        guard.last_sync_at_ms = Some(now_ms());
    }
    guard.last_error = enum_error.or(scan_error);
    // 总记录数本地可查，同步失败也照常刷新
    if let Ok(total) = db.with_conn(|c| (WSL_SOURCE.count)(c)) {
        guard.total_records = total;
    }
}

/// 扫描一个数据根（sessions/ 与 archived_sessions/）下所有 rollout 文件，
/// 逐个增量导入。返回本轮最后一个单文件错误（全部成功为 None）。
fn sync_root(db: &Database, root: &Path, source: &Source) -> Option<String> {
    let mut files = Vec::new();
    collect_jsonl(&root.join("sessions"), &mut files);
    collect_jsonl(&root.join("archived_sessions"), &mut files);
    let mut error = None;
    for f in &files {
        if let Err(e) = sync_file(db, f, source) {
            error = Some(format!("{}: {e}", f.display()));
        }
    }
    error
}

/// Codex 数据目录：Windows 上固定在用户配置目录下。
fn codex_root() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|p| PathBuf::from(p).join(".codex"))
}

/// 发行版内的 Codex 数据目录：家目录段按发行版真实 $HOME 解析
/// （见 wsl::unc_home_candidates，\\wsl$ 优先、\\wsl.localhost 兜底），
/// 只认第一个能读到目录的候选；都读不到返回 None（不算错误）。
fn wsl_codex_root(distro: &str) -> Option<PathBuf> {
    wsl::unc_home_candidates(distro, ".codex")
        .into_iter()
        .find(|p| p.is_dir())
}

/// 发行版列表缓存：wsl.exe -l -q 结果缓存 60 秒，避免每 15 秒一轮的扫描
/// 都拉起 wsl.exe。失败结果同样缓存（WSL 未就绪时不必每轮重试）。
fn cached_distros() -> Result<Vec<String>, String> {
    const TTL: Duration = Duration::from_secs(60);
    static CACHE: Mutex<Option<(Instant, Result<Vec<String>, String>)>> = Mutex::new(None);
    let mut guard = match CACHE.lock() {
        Ok(g) => g,
        Err(_) => return wsl::list_distros(),
    };
    if let Some((at, result)) = guard.as_ref() {
        if at.elapsed() < TTL {
            return result.clone();
        }
    }
    let result = wsl::list_distros();
    *guard = Some((Instant::now(), result.clone()));
    result
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
/// 注意：session_meta 不含模型名，模型由 meta["<源模型前缀>:<path>"] 恢复。
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
fn sync_file(db: &Database, path: &Path, source: &Source) -> Result<usize, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开失败: {e}"))?;
    let size = file
        .metadata()
        .map_err(|e| format!("读取元数据失败: {e}"))?
        .len();

    let offset_key = format!("{}{}", source.offset_prefix, path.display());
    let model_key = format!("{}{}", source.model_prefix, path.display());
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

    let inserted = db.with_conn_mut(|c| (source.insert)(c, &rows))?;
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
        agent: String::new(), // codex_events / wsl_codex_events 无 agent 列，仅占位
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
