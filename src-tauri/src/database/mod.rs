//! SQLite 数据层（SSOT）。库文件位于 app_data_dir/app.db。
//! 连接用 Arc<Mutex<Connection>> 共享：rusqlite 的 Connection 非 Sync，
//! 锁内操作都是毫秒级，持锁时间极短。

pub mod dao;
pub mod migrations;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建数据目录 {}: {e}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| format!("无法打开数据库 {}: {e}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(|e| format!("数据库 PRAGMA 设置失败: {e}"))?;
        migrations::run(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 以只读语义访问连接。
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let guard = self.conn.lock().map_err(|_| "数据库锁已损坏".to_string())?;
        f(&guard)
    }

    /// 以可写语义访问连接（事务需要 &mut Connection）。
    pub fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self.conn.lock().map_err(|_| "数据库锁已损坏".to_string())?;
        f(&mut guard)
    }
}
