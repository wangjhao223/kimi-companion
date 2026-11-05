//! IPC 命令层：统计聚合与模型清单。与 launcher/ledger 命令同一模式：
//! 耗时操作放进 spawn_blocking，错误以 String 返回前端。

use tauri::State;

use crate::database::{dao, Database};
use std::collections::HashMap;

use crate::models::{
    HeatmapDay, ModelCost, ModelPricing, RatePoint, SpeedStats, StatsSummary, TrendPoint,
};
use crate::services::{cost, model_catalog, stats};

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

/// 输出速度：以每个模型最近完成记录为基准，生成区间重叠的会话速度求和，
/// 最近使用的模型排最前（tok/s）。
#[tauri::command]
pub async fn get_stats_speed(db: State<'_, Database>) -> Result<SpeedStats, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || stats::get_speed(&db))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 可用模型清单：source=kimi 走 WSL（必须带 distro），source=kimi-win 直接读取
/// Windows Kimi Code 的活动 config.toml；只返回模型名。Codex 没有 provider 清单。
#[tauri::command]
pub async fn get_available_models(
    source: String,
    distro: Option<String>,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        model_catalog::get_available_models(&source, distro.as_deref())
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}

/// 按模型价格表逐事件计算费用，支持每日时段与上下文区间条件规则。
#[tauri::command]
pub async fn get_cost_estimates(
    db: State<'_, Database>,
    source: String,
    pricing: HashMap<String, ModelPricing>,
) -> Result<Vec<ModelCost>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let source = match source.as_str() {
            "kimi" => dao::SpeedSource::Kimi,
            "codex" => dao::SpeedSource::Codex,
            "kimi-win" => dao::SpeedSource::WinKimi,
            "wsl-codex" => dao::SpeedSource::WslCodex,
            other => return Err(format!("不支持的费用数据来源: {other}")),
        };
        cost::calculate(&db, source, &pricing)
    })
    .await
    .map_err(|e| format!("任务执行失败: {e}"))?
}
