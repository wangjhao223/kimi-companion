//! StatsService：usage_events 的聚合查询。
//!
//! 聚合全部走 SQL（见 dao.rs 的时区说明：本地时区天界，周一为一周开始）。

use crate::database::{dao, Database};
use crate::models::{
    HeatmapDay, ModelUsage, PeriodUsage, StatsSummary, TrendPoint,
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

fn to_model_usage(rows: Vec<(String, dao::UsageSums)>) -> Vec<ModelUsage> {
    rows.into_iter()
        .map(|(model, s)| ModelUsage {
            model,
            input: s.0,
            output: s.1,
            cache_read: s.2,
            cache_creation: s.3,
        })
        .collect()
}

/// 今日 / 本周（周一起）/ 本月 / 总计 + 按模型分组（总计与三个周期）。
pub fn get_summary(db: &Database) -> Result<StatsSummary, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        Ok(StatsSummary {
            today: to_period(dao::sum_usage(conn, Some(today_start))?),
            this_week: to_period(dao::sum_usage(conn, Some(week_start))?),
            this_month: to_period(dao::sum_usage(conn, Some(month_start))?),
            total: to_period(dao::sum_usage(conn, None)?),
            by_model: to_model_usage(dao::sum_by_model(conn)?),
            today_by_model: to_model_usage(dao::sum_by_model_since(conn, today_start)?),
            this_week_by_model: to_model_usage(dao::sum_by_model_since(conn, week_start)?),
            this_month_by_model: to_model_usage(dao::sum_by_model_since(conn, month_start)?),
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

/// Codex 今日 / 本周（周一起）/ 本月 / 总计 + 按模型分组（总计与三个周期）。
pub fn get_codex_summary(db: &Database) -> Result<StatsSummary, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        Ok(StatsSummary {
            today: to_period(dao::codex_sum_usage(conn, Some(today_start))?),
            this_week: to_period(dao::codex_sum_usage(conn, Some(week_start))?),
            this_month: to_period(dao::codex_sum_usage(conn, Some(month_start))?),
            total: to_period(dao::codex_sum_usage(conn, None)?),
            by_model: to_model_usage(dao::codex_sum_by_model(conn)?),
            today_by_model: to_model_usage(dao::codex_sum_by_model_since(conn, today_start)?),
            this_week_by_model: to_model_usage(dao::codex_sum_by_model_since(conn, week_start)?),
            this_month_by_model: to_model_usage(dao::codex_sum_by_model_since(conn, month_start)?),
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

// ---------- Windows Kimi Code 桌面端聚合（数据源 win_kimi_events，结构与 codex 侧一致） ----------

/// Win Kimi 今日 / 本周（周一起）/ 本月 / 总计 + 按模型分组（总计与三个周期）。
pub fn get_win_kimi_summary(db: &Database) -> Result<StatsSummary, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        Ok(StatsSummary {
            today: to_period(dao::win_kimi_sum_usage(conn, Some(today_start))?),
            this_week: to_period(dao::win_kimi_sum_usage(conn, Some(week_start))?),
            this_month: to_period(dao::win_kimi_sum_usage(conn, Some(month_start))?),
            total: to_period(dao::win_kimi_sum_usage(conn, None)?),
            by_model: to_model_usage(dao::win_kimi_sum_by_model(conn)?),
            today_by_model: to_model_usage(dao::win_kimi_sum_by_model_since(conn, today_start)?),
            this_week_by_model: to_model_usage(dao::win_kimi_sum_by_model_since(conn, week_start)?),
            this_month_by_model: to_model_usage(dao::win_kimi_sum_by_model_since(conn, month_start)?),
        })
    })
}

/// Win Kimi 热力图：近 days 天按天聚合的稀疏数组（默认 365 天）。
pub fn get_win_kimi_heatmap(db: &Database, days: Option<u32>) -> Result<Vec<HeatmapDay>, String> {
    let days = days.unwrap_or(HEATMAP_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::win_kimi_daily_sums(conn, days)?
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

/// Win Kimi 趋势：近 days 天按天三序列（默认 30 天，缺口补零）。
pub fn get_win_kimi_trend(db: &Database, days: Option<u32>) -> Result<Vec<TrendPoint>, String> {
    let days = days.unwrap_or(TREND_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::win_kimi_trend_series(conn, days)?
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

// ---------- WSL 侧 Codex 聚合（数据源 wsl_codex_events，结构与 codex 侧一致） ----------

/// WSL Codex 今日 / 本周（周一起）/ 本月 / 总计 + 按模型分组（总计与三个周期）。
pub fn get_wsl_codex_summary(db: &Database) -> Result<StatsSummary, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        Ok(StatsSummary {
            today: to_period(dao::wsl_codex_sum_usage(conn, Some(today_start))?),
            this_week: to_period(dao::wsl_codex_sum_usage(conn, Some(week_start))?),
            this_month: to_period(dao::wsl_codex_sum_usage(conn, Some(month_start))?),
            total: to_period(dao::wsl_codex_sum_usage(conn, None)?),
            by_model: to_model_usage(dao::wsl_codex_sum_by_model(conn)?),
            today_by_model: to_model_usage(dao::wsl_codex_sum_by_model_since(conn, today_start)?),
            this_week_by_model: to_model_usage(dao::wsl_codex_sum_by_model_since(conn, week_start)?),
            this_month_by_model: to_model_usage(dao::wsl_codex_sum_by_model_since(conn, month_start)?),
        })
    })
}

/// WSL Codex 热力图：近 days 天按天聚合的稀疏数组（默认 365 天）。
pub fn get_wsl_codex_heatmap(db: &Database, days: Option<u32>) -> Result<Vec<HeatmapDay>, String> {
    let days = days.unwrap_or(HEATMAP_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::wsl_codex_daily_sums(conn, days)?
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

/// WSL Codex 趋势：近 days 天按天三序列（默认 30 天，缺口补零）。
pub fn get_wsl_codex_trend(db: &Database, days: Option<u32>) -> Result<Vec<TrendPoint>, String> {
    let days = days.unwrap_or(TREND_DEFAULT_DAYS);
    db.with_conn(|conn| {
        Ok(dao::wsl_codex_trend_series(conn, days)?
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
