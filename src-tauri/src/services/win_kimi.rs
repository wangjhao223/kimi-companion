//! WinKimiService：把 Windows 原生 Kimi Code 桌面端的 wire.jsonl 增量导入 SQLite（win_kimi_events）。
//!
//! - 数据源：`%USERPROFILE%\.kimi-code\sessions\**\wire.jsonl`（递归，文件名精确匹配）；
//!   与 WSL 侧 hook 解析的是同一格式，但桌面端不装 hook，直接扫文件；
//! - 每个文件按 meta["win_kimi_off:<path>"] 字节偏移增量读，尾部残缺行留到下轮；
//! - offset 超过文件大小（文件被重建）回 0 重读，靠 UNIQUE + INSERT OR IGNORE 去重；
//! - 行格式：顶层 `type` 为 `usage.record` 的事件，字段映射与 hook/companion-hook.py
//!   完全一致：ts=event.time（毫秒）、session_id=event.sessionId（缺省回退到
//!   wire.jsonl 上三级目录名，即 sessions/<wd>/<session_id>/agents/<agent>/wire.jsonl
//!   中的 <session_id>）、model=event.model、input=usage.inputOther、
//!   output=usage.output、cache_read=usage.inputCacheRead、
//!   cache_creation=usage.inputCacheCreation；agentId 不入库（win_kimi_events 无 agent 列）；
//! - 解析规则变更时递增 PARSER_VERSION，启动迁移会清空偏移全量重读；
//! - 后台线程每 15 秒一轮，首轮全量 backfill；所有错误只进 WinKimiStatus，线程不 panic。

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::database::{dao, Database};
use crate::models::{SharedWinKimiStatus, WinKimiStatus};

const SYNC_INTERVAL: Duration = Duration::from_secs(15);
const OFFSET_KEY_PREFIX: &str = "win_kimi_off:";
/// wire.jsonl 解析规则版本：格式适配变更时 +1，触发一次全量重读。
const PARSER_VERSION: &str = "1";

pub fn new_shared_status() -> SharedWinKimiStatus {
    Arc::new(Mutex::new(WinKimiStatus::default()))
}

/// 启动后台同步线程。detached，不 join，随进程退出结束。
pub fn spawn_sync_thread(db: Database, status: SharedWinKimiStatus) {
    std::thread::Builder::new()
        .name("win-kimi-sync".to_string())
        .spawn(move || {
            migrate_offsets(&db);
            loop {
                sync_round(&db, &status);
                std::thread::sleep(SYNC_INTERVAL);
            }
        })
        .expect("无法启动 win_kimi 同步线程");
}

/// 解析器版本迁移：版本不符时清空所有文件偏移，下一轮即全量重读
/// （UNIQUE + INSERT OR IGNORE 保证重读不重复计数，无需清表）。
fn migrate_offsets(db: &Database) {
    let current = db
        .with_conn(|c| dao::get_meta(c, "win_kimi_parser_v"))
        .ok()
        .flatten();
    if current.as_deref() == Some(PARSER_VERSION) {
        return;
    }
    let _ = db.with_conn(|c| dao::delete_meta_prefix(c, OFFSET_KEY_PREFIX));
    let _ = db.with_conn(|c| dao::set_meta(c, "win_kimi_parser_v", PARSER_VERSION));
}

/// 一轮同步：扫 sessions 根下所有 wire.jsonl，逐个增量导入。
pub fn sync_round(db: &Database, status: &SharedWinKimiStatus) {
    let root = win_kimi_root().filter(|p| p.is_dir());
    let mut error: Option<String> = None;

    if let Some(root) = &root {
        let mut files = Vec::new();
        collect_wire_files(root, &mut files);
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
        Some("未找到 Kimi Code 桌面端数据目录（Kimi Code 桌面端未安装或尚未使用过）".to_string())
    };
    // 总记录数本地可查，同步失败也照常刷新
    if let Ok(total) = db.with_conn(|c| dao::count_win_kimi_events(c)) {
        guard.total_records = total;
    }
}

/// 桌面端数据目录：Windows 上固定在用户配置目录下。
fn win_kimi_root() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|p| PathBuf::from(p).join(".kimi-code").join("sessions"))
}

/// 递归收集 dir 下所有名为 wire.jsonl 的文件。目录不存在时静默返回空。
fn collect_wire_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_wire_files(&path, out);
        } else if path.file_name().is_some_and(|n| n == "wire.jsonl") {
            out.push(path);
        }
    }
}

/// 同步单个 wire.jsonl 的增量，返回本轮新插入条数。
fn sync_file(db: &Database, path: &Path) -> Result<usize, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开失败: {e}"))?;
    let size = file
        .metadata()
        .map_err(|e| format!("读取元数据失败: {e}"))?
        .len();

    let offset_key = format!("{OFFSET_KEY_PREFIX}{}", path.display());
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

    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("seek 失败: {e}"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("读取失败: {e}"))?;

    // 只消费到最后一个换行符：尾部可能是桌面端正在写入的残缺行，留到下轮。
    // 若把残缺行当垃圾跳过且偏移越过它，这条记录将永久丢失。
    let complete_len = match buf.iter().rposition(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => return Ok(0),
    };

    let text = String::from_utf8_lossy(&buf[..complete_len]);
    // 会话 id 回退：sessions/<wd>/<session_id>/agents/<agent>/wire.jsonl 中的 <session_id>
    let fallback_id = path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rows: Vec<dao::UsageEventRow> = Vec::new();
    for line in text.lines() {
        if !has_type(line, "usage.record") {
            continue;
        }
        if let Some(row) = parse_usage_line(line, &fallback_id) {
            rows.push(row);
        }
    }

    let inserted = db.with_conn_mut(|c| dao::insert_win_kimi_events(c, &rows))?;
    let new_offset = offset + complete_len as u64;
    db.with_conn(|c| dao::set_meta(c, &offset_key, &new_offset.to_string()))?;
    Ok(inserted)
}

/// 行类型预筛：同时兼容紧凑（"type":"x"）与带空格（"type": "x"）两种序列化。
/// 只是快速过滤，真正的字段校验在 serde 解析后做。
fn has_type(line: &str, t: &str) -> bool {
    line.contains(&format!("\"type\":\"{t}\"")) || line.contains(&format!("\"type\": \"{t}\""))
}

/// 宽松解析 usage.record 行（与 companion-hook.py 同规则）：缺 ts 或 usage 体则跳过。
fn parse_usage_line(line: &str, fallback_id: &str) -> Option<dao::UsageEventRow> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "usage.record" {
        return None;
    }
    let ts = v.get("time")?.as_i64()?;
    let u = v.get("usage")?;
    let n = |k: &str| u.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
    let session_id = v
        .get("sessionId")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_id);
    Some(dao::UsageEventRow {
        ts,
        session_id: session_id.to_string(),
        agent: String::new(), // win_kimi_events 无 agent 列，仅占位
        model: v
            .get("model")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        input_tokens: n("inputOther"),
        output_tokens: n("output"),
        cache_read: n("inputCacheRead"),
        cache_creation: n("inputCacheCreation"),
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
