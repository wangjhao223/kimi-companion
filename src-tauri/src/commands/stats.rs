//! IPC 命令层：统计聚合 + 配额查询。与 launcher/ledger 命令同一模式：
//! 耗时操作放进 spawn_blocking，错误以 String 返回前端。

use tauri::State;

use crate::commands::launcher::SharedInstance;
use crate::database::Database;
use crate::models::{HeatmapDay, QuotaInfo, StatsSummary, TrendPoint};
use crate::services::stats;

#[tauri::command]
pub async fn get_stats_summary(db: State<'_, Database>) -> Result<StatsSummary, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_summary(&db))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_stats_heatmap(
    db: State<'_, Database>,
    days: Option<u32>,
) -> Result<Vec<HeatmapDay>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_heatmap(&db, days))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_stats_trend(
    db: State<'_, Database>,
    days: Option<u32>,
) -> Result<Vec<TrendPoint>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_trend(&db, days))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 配额查询：实例未知或请求失败时返回 Ok(None)，前端降级显示「配额未知」。
#[tauri::command]
pub async fn get_quota(
    state: State<'_, SharedInstance>,
) -> Result<Option<QuotaInfo>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_quota(&state))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}
