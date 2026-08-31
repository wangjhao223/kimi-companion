//! StatsService：usage_events 的聚合查询 + 配额透传。
//!
//! - 聚合全部走 SQL（见 dao.rs 的时区说明：本地时区天界，周一为一周开始）；
//! - get_quota 用 SharedInstance 里的 (port, token) 调 kimi web 的
//!   GET /api/v1/oauth/usage，实例未知或请求失败一律返回 Ok(None)，
//!   由前端降级显示「配额未知」（REST API 是实验性的，只作增强）。

use std::time::Duration;

use serde_json::Value;

use crate::commands::launcher::SharedInstance;
use crate::database::{dao, Database};
use crate::services::http::shared_client;
use crate::models::{
    HeatmapDay, ModelUsage, PeriodUsage, QuotaInfo, QuotaWindow, StatsSummary, TrendPoint,
};

pub const HEATMAP_DEFAULT_DAYS: u32 = 365;
pub const TREND_DEFAULT_DAYS: u32 = 30;

fn to_period(sums: dao::UsageSums) -> PeriodUsage {
    PeriodUsage {
        input: sums.0,
        output: sums.1,
        cache_read: sums.2,
        cache_creation: sums.3,
    }
}

/// 今日 / 本周（周一起）/ 本月 / 总计 + 按模型分组。
pub fn get_summary(db: &Database) -> Result<StatsSummary, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        let by_model = dao::sum_by_model(conn)?
            .into_iter()
            .map(|(model, sums)| {
                let p = to_period(sums);
                ModelUsage {
                    model,
                    input: p.input,
                    output: p.output,
                    cache_read: p.cache_read,
                    cache_creation: p.cache_creation,
                }
            })
            .collect();
        Ok(StatsSummary {
            today: to_period(dao::sum_usage(conn, Some(today_start))?),
            this_week: to_period(dao::sum_usage(conn, Some(week_start))?),
            this_month: to_period(dao::sum_usage(conn, Some(month_start))?),
            total: to_period(dao::sum_usage(conn, None)?),
            by_model,
        })
    })
}

/// 热力图：近 days 天按天聚合的稀疏数组（默认 365 天，缺口由前端补零）。
pub fn get_heatmap(db: &Database, days: Option<u32>) -> Result<Vec<HeatmapDay>, String> {
    let days = days.unwrap_or(HEATMAP_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::daily_sums(conn, days)?
            .into_iter()
            .map(|(date, sums)| HeatmapDay {
                date,
                input: sums.0,
                output: sums.1,
                cache_read: sums.2,
                cache_creation: sums.3,
                total: sums.0 + sums.1 + sums.2 + sums.3,
            })
            .collect())
    })
}

/// 趋势：近 days 天按天三序列（默认 30 天，缺口补零）。
pub fn get_trend(db: &Database, days: Option<u32>) -> Result<Vec<TrendPoint>, String> {
    let days = days.unwrap_or(TREND_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::trend_series(conn, days)?
            .into_iter()
            .map(|(date, input, output, cache)| TrendPoint {
                date,
                input,
                output,
                cache,
            })
            .collect())
    })
}

// ---------- Codex 侧聚合（数据源 codex_events，结构与 kimi 侧一致） ----------

/// Codex 今日 / 本周（周一起）/ 本月 / 总计 + 按模型分组。
pub fn get_codex_summary(db: &Database) -> Result<StatsSummary, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        let by_model = dao::codex_sum_by_model(conn)?
            .into_iter()
            .map(|(model, sums)| {
                let p = to_period(sums);
                ModelUsage {
                    model,
                    input: p.input,
                    output: p.output,
                    cache_read: p.cache_read,
                    cache_creation: p.cache_creation,
                }
            })
            .collect();
        Ok(StatsSummary {
            today: to_period(dao::codex_sum_usage(conn, Some(today_start))?),
            this_week: to_period(dao::codex_sum_usage(conn, Some(week_start))?),
            this_month: to_period(dao::codex_sum_usage(conn, Some(month_start))?),
            total: to_period(dao::codex_sum_usage(conn, None)?),
            by_model,
        })
    })
}

/// Codex 热力图：近 days 天按天聚合的稀疏数组（默认 365 天）。
pub fn get_codex_heatmap(db: &Database, days: Option<u32>) -> Result<Vec<HeatmapDay>, String> {
    let days = days.unwrap_or(HEATMAP_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::codex_daily_sums(conn, days)?
            .into_iter()
            .map(|(date, sums)| HeatmapDay {
                date,
                input: sums.0,
                output: sums.1,
                cache_read: sums.2,
                cache_creation: sums.3,
                total: sums.0 + sums.1 + sums.2 + sums.3,
            })
            .collect())
    })
}

/// Codex 趋势：近 days 天按天三序列（默认 30 天，缺口补零）。
pub fn get_codex_trend(db: &Database, days: Option<u32>) -> Result<Vec<TrendPoint>, String> {
    let days = days.unwrap_or(TREND_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::codex_trend_series(conn, days)?
            .into_iter()
            .map(|(date, input, output, cache)| TrendPoint {
                date,
                input,
                output,
                cache,
            })
            .collect())
    })
}

/// 配额：透传 kimi web 的 /api/v1/oauth/usage。任何失败都返回 Ok(None)。
pub fn get_quota(state: &SharedInstance) -> Result<Option<QuotaInfo>, String> {
    let instance = state
        .lock()
        .map_err(|_| "应用状态锁已损坏".to_string())?
        .clone();
    let Some(inst) = instance else {
        return Ok(None);
    };

    let client = shared_client()?;
    let Ok(resp) = client
        .get(format!(
            "http://127.0.0.1:{}/api/v1/oauth/usage",
            inst.port
        ))
        .bearer_auth(&inst.token)
        .timeout(Duration::from_secs(3))
        .send()
    else {
        return Ok(None);
    };
    if !resp.status().is_success() {
        return Ok(None);
    }
    let Ok(v) = resp.json::<Value>() else {
        return Ok(None);
    };
    Ok(parse_quota(&v))
}

/// 宽松解析一个窗口：取 window.duration/window.unit/used/limit/reset_at，缺字段取零值。
fn parse_window(v: &Value) -> Option<QuotaWindow> {
    let window = v.get("window")?;
    Some(QuotaWindow {
        window_duration: window.get("duration").and_then(|x| x.as_i64()).unwrap_or(0),
        window_unit: window
            .get("unit")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        used: v.get("used").and_then(|x| x.as_f64()).unwrap_or(0.0),
        limit: v.get("limit").and_then(|x| x.as_f64()).unwrap_or(0.0),
        reset_at: v
            .get("reset_at")
            .and_then(|x| x.as_str())
            .map(String::from),
    })
}

/// 宽松解析整体响应：data.summary + data.limits[]；两者皆空视为无配额信息。
fn parse_quota(v: &Value) -> Option<QuotaInfo> {
    let data = v.get("data")?;
    let summary = data.get("summary").and_then(parse_window);
    let limits: Vec<QuotaWindow> = data
        .get("limits")
        .and_then(|l| l.as_array())
        .map(|arr| arr.iter().filter_map(parse_window).collect())
        .unwrap_or_default();
    if summary.is_none() && limits.is_empty() {
        None
    } else {
        Some(QuotaInfo { summary, limits })
    }
}
