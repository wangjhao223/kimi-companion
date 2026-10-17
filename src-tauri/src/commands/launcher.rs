//! IPC 命令层：启动页相关。耗时操作一律放进 spawn_blocking，避免阻塞主线程。

use std::sync::{Arc, Mutex};

use tauri::State;

use crate::models::{LaunchStatus, RunningInstance};
use crate::services::{launcher, wsl};

pub type SharedInstance = Arc<Mutex<Option<RunningInstance>>>;

#[tauri::command]
pub async fn list_distros() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(wsl::list_distros)
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_launch_status(
    state: State<'_, SharedInstance>,
    distro: Option<String>,
) -> Result<LaunchStatus, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || launcher::get_status(&state, distro.as_deref()))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))
}

#[tauri::command]
pub async fn start_kimi_web(
    state: State<'_, SharedInstance>,
    distro: String,
) -> Result<LaunchStatus, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || launcher::start(&state, &distro))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn stop_kimi_web(state: State<'_, SharedInstance>) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || launcher::stop(&state))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 开启/关闭 kimi web 远程控制（`kimi web --rc`），返回开关后的完整状态。
#[tauri::command]
pub async fn set_remote_control(
    state: State<'_, SharedInstance>,
    distro: String,
    enable: bool,
) -> Result<LaunchStatus, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        launcher::set_remote_control(&state, &distro, enable)
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 远程控制二维码 PNG 的 base64（WSL 内 ~/.kimi-code/rc-qrcode.png）。
#[tauri::command]
pub async fn get_rc_qrcode(distro: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || launcher::rc_qrcode_base64(&distro))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}
