//! IPC 命令层：统计聚合。与 launcher/ledger 命令同一模式：
//! 耗时操作放进 spawn_blocking，错误以 String 返回前端。

use tauri::State;

use crate::database::Database;
use crate::models::{HeatmapDay, RatePoint, SpeedStats, StatsSummary, TrendPoint};
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

/// 速率：近 minutes 分钟的 RPM/TPM 分钟序列（默认 60 分钟，缺口补零）。
#[tauri::command]
pub async fn get_stats_rates(
    db: State<'_, Database>,
    minutes: Option<u32>,
) -> Result<Vec<RatePoint>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_rates(&db, minutes))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。
#[tauri::command]
pub async fn get_stats_speed(db: State<'_, Database>) -> Result<SpeedStats, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_speed(&db))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}
