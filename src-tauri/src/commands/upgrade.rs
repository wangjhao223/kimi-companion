//! IPC 命令层：kimi CLI 版本查询与更新。耗时操作一律放进 spawn_blocking。

use tauri::State;

use crate::commands::launcher::SharedInstance;
use crate::models::{UpdateCheckResult, UpdateResult};
use crate::services::upgrade;

/// 查 kimi CLI 当前版本（WSL 内 `kimi --version`）。
#[tauri::command]
pub async fn get_kimi_cli_version(distro: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || upgrade::get_version(&distro))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 检查 kimi CLI 更新（WSL 内 `timeout 90 kimi upgrade`，不带 -y）。
#[tauri::command]
pub async fn check_kimi_cli_update(distro: String) -> Result<UpdateCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || upgrade::check_update(&distro))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 执行 kimi CLI 更新（WSL 内 `timeout 600 kimi upgrade -y`），
/// 成功后自动重启在跑的 kimi web。
#[tauri::command]
pub async fn update_kimi_cli(
    state: State<'_, SharedInstance>,
    distro: String,
) -> Result<UpdateResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || upgrade::update(&state, &distro))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}
