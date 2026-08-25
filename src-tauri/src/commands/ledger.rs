//! IPC 命令层：记账 hook 安装 + 账本同步状态查询。

use tauri::State;

use crate::commands::launcher::SharedInstance;
use crate::database::{dao, Database};
use crate::models::LedgerStatus;
use crate::services::{hook, launcher};
use crate::services::ledger::SharedLedgerStatus;

#[tauri::command]
pub async fn check_hook_installed(distro: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || hook::check_installed(&distro))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 一键安装记账 hook。返回是否重启了 kimi web：kimi web 只在启动时加载
/// hooks 配置，已在运行的实例必须重启才会开始记账（只重启 web，不动 TUI）。
#[tauri::command]
pub async fn install_hook(
    state: State<'_, SharedInstance>,
    distro: String,
) -> Result<bool, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook::install(&distro)?;
        launcher::restart_web_if_running(&state, &distro)
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 账本同步状态：内存里的最近同步时间/last error + 实时查库的总记录数。
#[tauri::command]
pub async fn get_ledger_status(
    db: State<'_, Database>,
    status: State<'_, SharedLedgerStatus>,
) -> Result<LedgerStatus, String> {
    let db = db.inner().clone();
    let status = status.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<LedgerStatus, String> {
        let total = db.with_conn(|c| dao::count_usage_events(c))?;
        let mut snapshot = status.lock().map_err(|_| "状态锁已损坏".to_string())?.clone();
        snapshot.total_records = total;
        Ok(snapshot)
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}
