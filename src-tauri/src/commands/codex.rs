//! IPC 命令层：Codex 侧统计聚合 + 同步状态。与 stats 命令同一模式：
//! 耗时操作放进 spawn_blocking，错误以 String 返回前端。

use tauri::State;

use crate::database::Database;
use crate::models::{CodexStatus, HeatmapDay, RatePoint, SpeedStats, StatsSummary, TrendPoint};
use crate::services::codex::SharedCodexStatus;
use crate::services::stats;

#[tauri::command]
pub async fn get_codex_summary(db: State<'_, Database>) -> Result<StatsSummary, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_codex_summary(&db))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_codex_heatmap(
    db: State<'_, Database>,
    days: Option<u32>,
) -> Result<Vec<HeatmapDay>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_codex_heatmap(&db, days))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

#[tauri::command]
pub async fn get_codex_trend(
    db: State<'_, Database>,
    days: Option<u32>,
) -> Result<Vec<TrendPoint>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_codex_trend(&db, days))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// Codex 速率：近 minutes 分钟的 RPM/TPM 分钟序列（默认 60 分钟，缺口补零）。
#[tauri::command]
pub async fn get_codex_rates(
    db: State<'_, Database>,
    minutes: Option<u32>,
) -> Result<Vec<RatePoint>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_codex_rates(&db, minutes))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// Codex 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。
#[tauri::command]
pub async fn get_codex_speed(db: State<'_, Database>) -> Result<SpeedStats, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_codex_speed(&db))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// Codex 同步状态：内存快照（目录/最近同步时间/last error），总记录数由同步线程刷新。
#[tauri::command]
pub async fn get_codex_status(
    status: State<'_, SharedCodexStatus>,
) -> Result<CodexStatus, String> {
    let status = status.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let guard = status.lock().map_err(|_| "状态锁已损坏".to_string())?;
        Ok(guard.clone())
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}
