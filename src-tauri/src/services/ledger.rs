//! LedgerService：把 WSL 内的 token-ledger.jsonl 增量同步进 SQLite。
//!
//! - 账本通过 UNC 路径读取（`\\wsl$\<distro>\...`，失败时回退 `\\wsl.localhost\...`）；
//! - 按 meta.ledger_offset 字节偏移增量读，尾部残缺行不消费（下轮补上）；
//! - offset 超过文件大小（账本被截断/轮换）时回到 0 从头读，靠 UNIQUE+IGNORE 去重；
//! - 后台线程每 30 秒同步一轮，首轮 offset=0 即自然完成全量 backfill；
//! - 所有错误只记录到内存状态（LedgerStatus.last_error），线程永不 panic。

use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::database::{dao, Database};
use crate::models::LedgerStatus;
use crate::services::wsl;

const META_LEDGER_OFFSET: &str = "ledger_offset";
const SYNC_INTERVAL: Duration = Duration::from_secs(30);

/// 内存中的同步状态，供 get_ledger_status 命令查询。
pub type SharedLedgerStatus = Arc<Mutex<LedgerStatus>>;

pub fn new_shared_status() -> SharedLedgerStatus {
    Arc::new(Mutex::new(LedgerStatus::default()))
}

/// 启动后台同步线程。detached，不 join，随进程退出结束。
pub fn spawn_sync_thread(db: Database, status: SharedLedgerStatus) {
    std::thread::Builder::new()
        .name("ledger-sync".to_string())
        .spawn(move || loop {
            sync_round(&db, &status);
            std::thread::sleep(SYNC_INTERVAL);
        })
        .expect("无法启动账本同步线程");
}

/// 联动触发端口：hook 写完账本后 POST 一下，应用立即同步一轮。
pub const SYNC_TRIGGER_PORT: u16 = 51999;

/// 监听 127.0.0.1:SYNC_TRIGGER_PORT，任意连接（ hook 的 POST ）触发一轮立即同步。
///
/// 只绑 loopback：WSL2 镜像网络模式下 WSL 内可经 127.0.0.1 到达 Windows 侧；
/// NAT 模式下 hook 够不到这里，自然退回 30 秒轮询兜底。端口被占时同样退回
/// 纯轮询。30 秒轮询线程始终存在，本监听只是加速通道。
pub fn spawn_trigger_listener(db: Database, status: SharedLedgerStatus) {
    std::thread::Builder::new()
        .name("ledger-sync-trigger".to_string())
        .spawn(move || {
            let listener =
                match std::net::TcpListener::bind(("127.0.0.1", SYNC_TRIGGER_PORT)) {
                    Ok(l) => l,
                    Err(_) => return,
                };
            for stream in listener.incoming() {
                // 请求内容无需解析（只支持 POST /sync 这一语义），回个 200 即可
                if let Ok(mut s) = stream {
                    use std::io::Write;
                    let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
                    sync_round(&db, &status);
                }
            }
        })
        .expect("无法启动账本同步触发监听");
}

/// 一轮同步：遍历所有发行版，第一个同步成功即停（账本只会在装了 hook 的发行版里）。
pub fn sync_round(db: &Database, status: &SharedLedgerStatus) {
    let result = wsl::list_distros().and_then(|distros| {
        let mut last_err: Option<String> = None;
        for distro in &distros {
            match sync_distro(db, distro) {
                Ok(_) => return Ok(()),
                Err(e) => last_err = Some(e),
            }
        }
        match last_err {
            Some(e) => Err(e),
            None => Err("没有可用的 WSL 发行版".to_string()),
        }
    });

    let mut guard = match status.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    match result {
        Ok(()) => {
            guard.last_sync_at_ms = Some(now_ms());
            guard.last_error = None;
        }
        Err(e) => guard.last_error = Some(e),
    }
    // 总记录数本地可查，同步失败也照常刷新
    if let Ok(total) = db.with_conn(|c| dao::count_usage_events(c)) {
        guard.total_records = total;
    }
}

/// 同步单个发行版的账本增量，返回本轮新插入条数。
pub fn sync_distro(db: &Database, distro: &str) -> Result<usize, String> {
    let mut file = open_ledger(distro)?;
    let size = file
        .metadata()
        .map_err(|e| format!("读取账本元数据失败: {e}"))?
        .len();

    let mut offset: u64 = db
        .with_conn(|c| dao::get_meta(c, META_LEDGER_OFFSET))?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if offset > size {
        // 账本被截断或重建，从头重读（INSERT OR IGNORE 保证不重复）
        offset = 0;
    }
    if offset == size {
        return Ok(0);
    }

    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("账本 seek 失败: {e}"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| format!("读取账本失败: {e}"))?;

    // 只消费到最后一个换行符：尾部可能是 hook 正在写入的残缺行，留到下轮
    let complete_len = match buf.iter().rposition(|&b| b == b'\n') {
        Some(pos) => pos + 1,
        None => return Ok(0),
    };

    let text = String::from_utf8_lossy(&buf[..complete_len]);
    let rows: Vec<dao::UsageEventRow> = text.lines().filter_map(parse_line).collect();

    let inserted = db.with_conn_mut(|c| dao::insert_usage_events(c, &rows))?;
    let new_offset = offset + complete_len as u64;
    db.with_conn(|c| dao::set_meta(c, META_LEDGER_OFFSET, &new_offset.to_string()))?;
    Ok(inserted)
}

/// 依次尝试 `\\wsl$` 和 `\\wsl.localhost` 两种 UNC 前缀打开账本。
fn open_ledger(distro: &str) -> Result<std::fs::File, String> {
    let candidates = [
        format!(r"\\wsl$\{distro}\root\.kimi-code\token-ledger.jsonl"),
        format!(r"\\wsl.localhost\{distro}\root\.kimi-code\token-ledger.jsonl"),
    ];
    let mut last_err = String::new();
    for path in &candidates {
        match std::fs::File::open(path) {
            Ok(f) => return Ok(f),
            Err(e) => last_err = format!("{path}: {e}"),
        }
    }
    Err(format!("无法打开账本文件: {last_err}"))
}

/// 宽松解析一行账本 JSON：缺 ts 或整体非法则跳过，其余字段缺失取零值/空串。
fn parse_line(line: &str) -> Option<dao::UsageEventRow> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let ts = v.get("ts")?.as_i64()?;
    let s = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    let n = |k: &str| v.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
    Some(dao::UsageEventRow {
        ts,
        session_id: s("session_id"),
        agent: s("agent"),
        model: s("model"),
        input_tokens: n("input"),
        output_tokens: n("output"),
        cache_read: n("cache_read"),
        cache_creation: n("cache_creation"),
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
