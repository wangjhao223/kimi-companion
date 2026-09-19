//! IPC 命令层：Windows 原生 Kimi Code 桌面端统计聚合 + 同步状态 + 桌面应用管理。
//! 与 codex 命令同一模式：耗时操作放进 spawn_blocking，错误以 String 返回前端。

use tauri::State;

use crate::database::Database;
use crate::models::{
    DesktopAppInfo, HeatmapDay, SharedWinKimiStatus, StatsSummary, TrendPoint, WinKimiStatus,
};
use crate::services::{desktop, stats};

#[tauri::command]
pub async fn get_win_kimi_summary(db: State<'_, Database>) -> Result<StatsSummary, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_win_kimi_summary(&db))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_win_kimi_heatmap(
    db: State<'_, Database>,
    days: Option<u32>,
) -> Result<Vec<HeatmapDay>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_win_kimi_heatmap(&db, days))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_win_kimi_trend(
    db: State<'_, Database>,
    days: Option<u32>,
) -> Result<Vec<TrendPoint>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_win_kimi_trend(&db, days))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// Win Kimi 同步状态：内存快照（目录/最近同步时间/last error），总记录数由同步线程刷新。
#[tauri::command]
pub async fn get_win_kimi_status(
    status: State<'_, SharedWinKimiStatus>,
) -> Result<WinKimiStatus, String> {
    let status = status.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = status.lock().map_err(|_| "状态锁已损坏".to_string())?;
        Ok(guard.clone())
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 桌面应用（Kimi Code）的安装 + 运行状态快照。
#[tauri::command]
pub async fn get_desktop_apps_status() -> Result<Vec<DesktopAppInfo>, String> {
    tauri::async_runtime::spawn_blocking(desktop::desktop_apps_status)
        .await
        .map_err(|e| format!("任务执行失败: {e}"))
}

/// 启动指定桌面应用。app 当前仅支持 "kimi-code"；未安装时返回中文错误。
#[tauri::command]
pub async fn start_desktop_app(app: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || desktop::start_app(&app))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}
