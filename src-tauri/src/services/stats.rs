//! StatsService：usage_events 的聚合查询。
//!
//! 聚合全部走 SQL（见 dao.rs 的时区说明：本地时区天界，周一为一周开始）。

use crate::database::{dao, Database};
use crate::models::{
    HeatmapDay, ModelUsage, PeriodUsage, RatePoint, SpeedStats, StatsSummary, TrendPoint,
};

pub const HEATMAP_DEFAULT_DAYS: u32 = 365;
pub const TREND_DEFAULT_DAYS: u32 = 30;
pub const RATE_DEFAULT_MINUTES: u32 = 60;

fn to_rate_points(rows: Vec<dao::MinuteRateRow>) -> Vec<RatePoint> {
    rows.into_iter()
        .map(|(minute_start_ms, requests, tokens)| RatePoint {
            minute_start_ms,
            requests,
            tokens,
        })
        .collect()
}

/// 输出速度统计的公共装配（四个数据源共用）：当前瞬时速度 + 近 1 小时 / 今日加权均速。
/// 加权均速 = Σoutput/Σduration（只统计带耗时的记录），无数据为 None。
fn speed_stats(
    db: &Database,
    latest_fn: fn(&rusqlite::Connection) -> Result<Option<(i64, i64)>, String>,
    avg_fn: fn(&rusqlite::Connection, i64) -> Result<(i64, i64), String>,
) -> Result<SpeedStats, String> {
    db.with_conn(|conn| {
        let (today_start, _, _) = dao::local_period_bounds(conn)?;
        let now_ms: i64 = conn
            .query_row("SELECT CAST(strftime('%s','now') AS INTEGER) * 1000", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读取当前时间失败: {e}"))?;
        let to_speed = |agg: (i64, i64)| {
            let (out, dur) = agg;
            if dur > 0 {
                Some(out as f64 / dur as f64 * 1000.0)
            } else {
                None
            }
        };
        Ok(SpeedStats {
            latest_speed: latest_fn(conn)?.and_then(|agg| to_speed(agg)),
            hour_avg_speed: to_speed(avg_fn(conn, now_ms - 3_600_000)?),
            today_avg_speed: to_speed(avg_fn(conn, today_start)?),
        })
    })
}

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

/// 速率：近 minutes 分钟按分钟桶的 RPM/TPM 序列（默认 60 分钟，缺口补零）。
pub fn get_rates(db: &Database, minutes: Option<u32>) -> Result<Vec<RatePoint>, String> {
    let minutes = minutes.unwrap_or(RATE_DEFAULT_MINUTES);
    db.with_conn(|conn| Ok(to_rate_points(dao::minute_rates(conn, minutes)?)))
}

/// 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。
pub fn get_speed(db: &Database) -> Result<SpeedStats, String> {
    speed_stats(db, dao::latest_speed, dao::speed_avg)
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

/// Codex 速率：近 minutes 分钟按分钟桶的 RPM/TPM 序列（默认 60 分钟，缺口补零）。
pub fn get_codex_rates(db: &Database, minutes: Option<u32>) -> Result<Vec<RatePoint>, String> {
    let minutes = minutes.unwrap_or(RATE_DEFAULT_MINUTES);
    db.with_conn(|conn| Ok(to_rate_points(dao::codex_minute_rates(conn, minutes)?)))
}

/// Codex 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。
pub fn get_codex_speed(db: &Database) -> Result<SpeedStats, String> {
    speed_stats(db, dao::codex_latest_speed, dao::codex_speed_avg)
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

/// Win Kimi 速率：近 minutes 分钟按分钟桶的 RPM/TPM 序列（默认 60 分钟，缺口补零）。
pub fn get_win_kimi_rates(db: &Database, minutes: Option<u32>) -> Result<Vec<RatePoint>, String> {
    let minutes = minutes.unwrap_or(RATE_DEFAULT_MINUTES);
    db.with_conn(|conn| Ok(to_rate_points(dao::win_kimi_minute_rates(conn, minutes)?)))
}

/// Win Kimi 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。
pub fn get_win_kimi_speed(db: &Database) -> Result<SpeedStats, String> {
    speed_stats(db, dao::win_kimi_latest_speed, dao::win_kimi_speed_avg)
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

/// WSL Codex 速率：近 minutes 分钟按分钟桶的 RPM/TPM 序列（默认 60 分钟，缺口补零）。
pub fn get_wsl_codex_rates(db: &Database, minutes: Option<u32>) -> Result<Vec<RatePoint>, String> {
    let minutes = minutes.unwrap_or(RATE_DEFAULT_MINUTES);
    db.with_conn(|conn| Ok(to_rate_points(dao::wsl_codex_minute_rates(conn, minutes)?)))
}

/// WSL Codex 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。
pub fn get_wsl_codex_speed(db: &Database) -> Result<SpeedStats, String> {
    speed_stats(db, dao::wsl_codex_latest_speed, dao::wsl_codex_speed_avg)
}
