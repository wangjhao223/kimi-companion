//! usage_events / meta 的读写。聚合查询（M3 StatsService）在此追加，不影响现有接口。

use rusqlite::{params, Connection, OptionalExtension};

/// 一条待写入的账本记录（对应 token-ledger.jsonl 的一行）。
#[derive(Debug, Clone)]
pub struct UsageEventRow {
    pub ts: i64,
    pub session_id: String,
    pub agent: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    /// 生成耗时毫秒（kimi 侧 = llm.request→usage.record；codex 侧 = 回合墙钟）。
    /// 采集端无法配对时为 None，历史行保持 NULL。
    pub duration_ms: Option<i64>,
}

/// 批量写入账本记录，单事务 + INSERT OR IGNORE 幂等，返回实际插入条数。
pub fn insert_usage_events(conn: &mut Connection, rows: &[UsageEventRow]) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {e}"))?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO usage_events
                 (ts, session_id, agent, model, input_tokens, output_tokens, cache_read, cache_creation, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .map_err(|e| format!("预编译插入语句失败: {e}"))?;
        for row in rows {
            inserted += stmt
                .execute(params![
                    row.ts,
                    row.session_id,
                    row.agent,
                    row.model,
                    row.input_tokens,
                    row.output_tokens,
                    row.cache_read,
                    row.cache_creation,
                    row.duration_ms,
                ])
                .map_err(|e| format!("写入 usage_events 失败: {e}"))?;
        }
    }
    tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
    Ok(inserted)
}

/// usage_events 总记录数。
pub fn count_usage_events(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))
        .map_err(|e| format!("统计 usage_events 失败: {e}"))
}

// ---------- M3：聚合查询 ----------
//
// 时区处理：天界一律按本地时区。'now','localtime' 把 UTC 时间值改写成
// 本地墙钟，'start of day'/'start of month' 取整后，末尾的 'utc' 再按
// 本地时区解释回去，得到真正的 UTC 毫秒边界（%s 输出的是纪元秒）。
// 分组用 date(ts/1000,'unixepoch','localtime') 直接落在本地日期上。

/// 四列合计的类型别名：(input, output, cache_read, cache_creation)。
pub type UsageSums = (i64, i64, i64, i64);

const SUM_COLS: &str =
    "COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), \
     COALESCE(SUM(cache_read),0), COALESCE(SUM(cache_creation),0)";

fn read_sums(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<UsageSums> {
    Ok((
        row.get(offset)?,
        row.get(offset + 1)?,
        row.get(offset + 2)?,
        row.get(offset + 3)?,
    ))
}

/// 本地时区的三个天界：(今日零点 ms, 本周一零点 ms, 本月一号零点 ms)。
pub fn local_period_bounds(conn: &Connection) -> Result<(i64, i64, i64), String> {
    let (today_start, days_since_monday, month_start): (i64, i64, i64) = conn
        .query_row(
            "SELECT \
                CAST(strftime('%s','now','localtime','start of day','utc') AS INTEGER) * 1000, \
                (CAST(strftime('%w','now','localtime') AS INTEGER) + 6) % 7, \
                CAST(strftime('%s','now','localtime','start of month','utc') AS INTEGER) * 1000",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| format!("计算本地天界失败: {e}"))?;
    // %w：0=周日..6=周六；+6 再 mod 7 即「距本周一过了几天」（周一为一周开始）
    let week_start = today_start - days_since_monday * 86_400_000;
    Ok((today_start, week_start, month_start))
}

/// 时间段合计；since_ms 为 None 表示全部记录。
pub fn sum_usage(conn: &Connection, since_ms: Option<i64>) -> Result<UsageSums, String> {
    let result = match since_ms {
        Some(since) => conn.query_row(
            &format!("SELECT {SUM_COLS} FROM usage_events WHERE ts >= ?1"),
            [since],
            |r| read_sums(r, 0),
        ),
        None => conn.query_row(&format!("SELECT {SUM_COLS} FROM usage_events"), [], |r| {
            read_sums(r, 0)
        }),
    };
    result.map_err(|e| format!("聚合用量失败: {e}"))
}

/// 按模型分组合计，按总量降序。
pub fn sum_by_model(conn: &Connection) -> Result<Vec<(String, UsageSums)>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT model, {SUM_COLS} FROM usage_events GROUP BY model \
             ORDER BY SUM(input_tokens)+SUM(output_tokens)+SUM(cache_read)+SUM(cache_creation) DESC"
        ))
        .map_err(|e| format!("预编译模型聚合失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("模型聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取模型聚合结果失败: {e}"))
}

/// 时间段内按模型分组合计（ts >= since_ms），按总量降序。
/// table 只传代码内常量（三张表共用这一套 SQL），无注入面。
fn sum_by_model_since_on(
    conn: &Connection,
    table: &str,
    since_ms: i64,
) -> Result<Vec<(String, UsageSums)>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT model, {SUM_COLS} FROM {table} WHERE ts >= ?1 GROUP BY model \
             ORDER BY SUM(input_tokens)+SUM(output_tokens)+SUM(cache_read)+SUM(cache_creation) DESC"
        ))
        .map_err(|e| format!("预编译模型聚合失败: {e}"))?;
    let rows = stmt
        .query_map([since_ms], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("模型聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取模型聚合结果失败: {e}"))
}

/// usage_events 时间段内按模型分组合计。
pub fn sum_by_model_since(
    conn: &Connection,
    since_ms: i64,
) -> Result<Vec<(String, UsageSums)>, String> {
    sum_by_model_since_on(conn, "usage_events", since_ms)
}

/// 按天聚合的稀疏数组（只有数据的天），近 days 天，按日期升序。
/// 返回 (date, input, output, cache_read, cache_creation)。
pub fn daily_sums(conn: &Connection, days: u32) -> Result<Vec<(String, UsageSums)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT date(ts/1000,'unixepoch','localtime') d, {SUM_COLS} \
             FROM usage_events \
             WHERE ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY d ORDER BY d"
        ))
        .map_err(|e| format!("预编译按天聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("按天聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取按天聚合结果失败: {e}"))
}

/// 按天三序列（输入/输出/缓存，缓存 = cache_read + cache_creation），
/// 近 days 天，递归 CTE 生成完整日期轴再 LEFT JOIN，缺口自动补零。
/// ON 子句带 ts 范围条件走 idx_usage_day 索引，否则 JOIN 键上的 date()
/// 表达式会对全表逐行求值（数据量大了以后每 30s 轮询都是全表扫）。
pub fn trend_series(conn: &Connection, days: u32) -> Result<Vec<(String, i64, i64, i64)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE days(d) AS ( \
               SELECT date('now','localtime', ?1) \
               UNION ALL \
               SELECT date(d,'+1 day') FROM days WHERE d < date('now','localtime') \
             ) \
             SELECT days.d, \
                COALESCE(SUM(u.input_tokens),0), \
                COALESCE(SUM(u.output_tokens),0), \
                COALESCE(SUM(u.cache_read),0) + COALESCE(SUM(u.cache_creation),0) \
             FROM days \
             LEFT JOIN usage_events u \
               ON date(u.ts/1000,'unixepoch','localtime') = days.d \
              AND u.ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY days.d ORDER BY days.d",
        )
        .map_err(|e| format!("预编译趋势聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|e| format!("趋势聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<(String, i64, i64, i64)>, _>>()
        .map_err(|e| format!("读取趋势聚合结果失败: {e}"))
}

/// 一分钟速率桶：(桶起点 ms, 请求数, token 合计)。token 合计 = 输入+输出+缓存读+缓存写。
pub type MinuteRateRow = (i64, i64, i64);

/// 按分钟聚合指定事件表的速率序列：近 minutes 分钟（含当前分钟），完整分钟轴缺口补零。
/// 桶界按绝对纪元毫秒对齐（ts/60000），与时区无关；表名为代码内部常量，无注入风险。
fn minute_series_on(
    conn: &Connection,
    table: &str,
    minutes: u32,
) -> Result<Vec<MinuteRateRow>, String> {
    let minutes = minutes.clamp(1, 24 * 60);
    let now_ms: i64 = conn
        .query_row("SELECT CAST(strftime('%s','now') AS INTEGER) * 1000", [], |r| {
            r.get(0)
        })
        .map_err(|e| format!("读取当前时间失败: {e}"))?;
    let cur_bucket = now_ms / 60000 * 60000;
    let since = cur_bucket - (minutes as i64 - 1) * 60000;

    let mut stmt = conn
        .prepare(&format!(
            "SELECT (ts/60000)*60000 b, COUNT(*), \
             COALESCE(SUM(input_tokens+output_tokens+cache_read+cache_creation),0) \
             FROM {table} WHERE ts >= ?1 GROUP BY b ORDER BY b"
        ))
        .map_err(|e| format!("预编译分钟聚合失败: {e}"))?;
    let sparse: std::collections::HashMap<i64, (i64, i64)> = stmt
        .query_map([since], |r| {
            Ok((r.get::<_, i64>(0)?, (r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))
        })
        .map_err(|e| format!("分钟聚合查询失败: {e}"))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("读取分钟聚合结果失败: {e}"))?;

    let mut out = Vec::with_capacity(minutes as usize);
    for i in 0..minutes as i64 {
        let b = since + i * 60000;
        let (req, tok) = sparse.get(&b).copied().unwrap_or((0, 0));
        out.push((b, req, tok));
    }
    Ok(out)
}

/// usage_events 分钟速率序列，近 minutes 分钟（含当前分钟），缺口补零。
pub fn minute_rates(conn: &Connection, minutes: u32) -> Result<Vec<MinuteRateRow>, String> {
    minute_series_on(conn, "usage_events", minutes)
}

/// 指定事件表在 since_ms 以来的速度聚合：(输出 token 合计, 生成耗时合计 ms)。
/// 加权均速 tok/s = output/duration*1000，由调用方计算；只统计 duration_ms
/// 非空的行（历史行与配对失败的行没有耗时，不参与速度计算）。
fn speed_avg_on(conn: &Connection, table: &str, since_ms: i64) -> Result<(i64, i64), String> {
    conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(output_tokens),0), COALESCE(SUM(duration_ms),0) \
             FROM {table} WHERE ts >= ?1 AND duration_ms IS NOT NULL"
        ),
        [since_ms],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| format!("速度聚合查询失败: {e}"))
}

/// 指定事件表最近一次带耗时的记录：(output_tokens, duration_ms)，供「当前速度」。
fn latest_speed_on(conn: &Connection, table: &str) -> Result<Option<(i64, i64)>, String> {
    conn.query_row(
        &format!(
            "SELECT output_tokens, duration_ms FROM {table} \
             WHERE duration_ms IS NOT NULL AND duration_ms > 0 \
             ORDER BY ts DESC LIMIT 1"
        ),
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
    .map_err(|e| format!("查询最近速度记录失败: {e}"))
}

/// usage_events 在 since_ms 以来的速度聚合。
pub fn speed_avg(conn: &Connection, since_ms: i64) -> Result<(i64, i64), String> {
    speed_avg_on(conn, "usage_events", since_ms)
}

/// usage_events 最近一次带耗时的记录。
pub fn latest_speed(conn: &Connection) -> Result<Option<(i64, i64)>, String> {
    latest_speed_on(conn, "usage_events")
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()
        .map_err(|e| format!("读取 meta[{key}] 失败: {e}"))
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("写入 meta[{key}] 失败: {e}"))?;
    Ok(())
}

/// 删除指定前缀的所有 meta 键（前缀由代码内部生成，不含 LIKE 通配符）。
pub fn delete_meta_prefix(conn: &Connection, prefix: &str) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM meta WHERE key LIKE ?1",
        [format!("{prefix}%")],
    )
    .map_err(|e| format!("按前缀删除 meta[{prefix}*] 失败: {e}"))
}

// ---------- Codex 侧：codex_events 的写入与聚合（与 usage_events 同构，无 agent 列） ----------

/// 批量写入 codex 用量记录，单事务 + INSERT OR IGNORE 幂等，返回实际插入条数。
/// 复用 UsageEventRow，agent 字段不写入（codex_events 无此列）。
pub fn insert_codex_events(conn: &mut Connection, rows: &[UsageEventRow]) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {e}"))?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO codex_events
                 (ts, session_id, model, input_tokens, output_tokens, cache_read, cache_creation, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| format!("预编译插入语句失败: {e}"))?;
        for row in rows {
            inserted += stmt
                .execute(params![
                    row.ts,
                    row.session_id,
                    row.model,
                    row.input_tokens,
                    row.output_tokens,
                    row.cache_read,
                    row.cache_creation,
                    row.duration_ms,
                ])
                .map_err(|e| format!("写入 codex_events 失败: {e}"))?;
        }
    }
    tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
    Ok(inserted)
}

/// codex_events 总记录数。
pub fn count_codex_events(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM codex_events", [], |r| r.get(0))
        .map_err(|e| format!("统计 codex_events 失败: {e}"))
}

/// codex 时间段合计；since_ms 为 None 表示全部记录。
pub fn codex_sum_usage(conn: &Connection, since_ms: Option<i64>) -> Result<UsageSums, String> {
    let result = match since_ms {
        Some(since) => conn.query_row(
            &format!("SELECT {SUM_COLS} FROM codex_events WHERE ts >= ?1"),
            [since],
            |r| read_sums(r, 0),
        ),
        None => conn.query_row(&format!("SELECT {SUM_COLS} FROM codex_events"), [], |r| {
            read_sums(r, 0)
        }),
    };
    result.map_err(|e| format!("聚合 codex 用量失败: {e}"))
}

/// codex 按模型分组合计，按总量降序。
pub fn codex_sum_by_model(conn: &Connection) -> Result<Vec<(String, UsageSums)>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT model, {SUM_COLS} FROM codex_events GROUP BY model \
             ORDER BY SUM(input_tokens)+SUM(output_tokens)+SUM(cache_read)+SUM(cache_creation) DESC"
        ))
        .map_err(|e| format!("预编译 codex 模型聚合失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("codex 模型聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取 codex 模型聚合结果失败: {e}"))
}

/// codex_events 时间段内按模型分组合计。
pub fn codex_sum_by_model_since(
    conn: &Connection,
    since_ms: i64,
) -> Result<Vec<(String, UsageSums)>, String> {
    sum_by_model_since_on(conn, "codex_events", since_ms)
}

/// codex 按天聚合的稀疏数组，近 days 天，按日期升序。
pub fn codex_daily_sums(conn: &Connection, days: u32) -> Result<Vec<(String, UsageSums)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT date(ts/1000,'unixepoch','localtime') d, {SUM_COLS} \
             FROM codex_events \
             WHERE ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY d ORDER BY d"
        ))
        .map_err(|e| format!("预编译 codex 按天聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("codex 按天聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取 codex 按天聚合结果失败: {e}"))
}

/// codex 按天三序列（输入/输出/缓存），近 days 天，缺口补零。
pub fn codex_trend_series(conn: &Connection, days: u32) -> Result<Vec<(String, i64, i64, i64)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE days(d) AS ( \
               SELECT date('now','localtime', ?1) \
               UNION ALL \
               SELECT date(d,'+1 day') FROM days WHERE d < date('now','localtime') \
             ) \
             SELECT days.d, \
                COALESCE(SUM(u.input_tokens),0), \
                COALESCE(SUM(u.output_tokens),0), \
                COALESCE(SUM(u.cache_read),0) + COALESCE(SUM(u.cache_creation),0) \
             FROM days \
             LEFT JOIN codex_events u \
               ON date(u.ts/1000,'unixepoch','localtime') = days.d \
              AND u.ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY days.d ORDER BY days.d",
        )
        .map_err(|e| format!("预编译 codex 趋势聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|e| format!("codex 趋势聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<(String, i64, i64, i64)>, _>>()
        .map_err(|e| format!("读取 codex 趋势聚合结果失败: {e}"))
}

/// codex_events 分钟速率序列，近 minutes 分钟（含当前分钟），缺口补零。
pub fn codex_minute_rates(conn: &Connection, minutes: u32) -> Result<Vec<MinuteRateRow>, String> {
    minute_series_on(conn, "codex_events", minutes)
}

/// codex_events 在 since_ms 以来的速度聚合。
pub fn codex_speed_avg(conn: &Connection, since_ms: i64) -> Result<(i64, i64), String> {
    speed_avg_on(conn, "codex_events", since_ms)
}

/// codex_events 最近一次带耗时的记录。
pub fn codex_latest_speed(conn: &Connection) -> Result<Option<(i64, i64)>, String> {
    latest_speed_on(conn, "codex_events")
}

// ---------- Windows Kimi Code 桌面端：win_kimi_events 的写入与聚合（与 codex_events 同构，无 agent 列） ----------

/// 批量写入 win_kimi 用量记录，单事务 + INSERT OR IGNORE 幂等，返回实际插入条数。
/// 复用 UsageEventRow，agent 字段不写入（win_kimi_events 无此列）。
pub fn insert_win_kimi_events(conn: &mut Connection, rows: &[UsageEventRow]) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {e}"))?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO win_kimi_events
                 (ts, session_id, model, input_tokens, output_tokens, cache_read, cache_creation, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| format!("预编译插入语句失败: {e}"))?;
        for row in rows {
            inserted += stmt
                .execute(params![
                    row.ts,
                    row.session_id,
                    row.model,
                    row.input_tokens,
                    row.output_tokens,
                    row.cache_read,
                    row.cache_creation,
                    row.duration_ms,
                ])
                .map_err(|e| format!("写入 win_kimi_events 失败: {e}"))?;
        }
    }
    tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
    Ok(inserted)
}

/// win_kimi_events 总记录数。
pub fn count_win_kimi_events(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM win_kimi_events", [], |r| r.get(0))
        .map_err(|e| format!("统计 win_kimi_events 失败: {e}"))
}

/// win_kimi 时间段合计；since_ms 为 None 表示全部记录。
pub fn win_kimi_sum_usage(conn: &Connection, since_ms: Option<i64>) -> Result<UsageSums, String> {
    let result = match since_ms {
        Some(since) => conn.query_row(
            &format!("SELECT {SUM_COLS} FROM win_kimi_events WHERE ts >= ?1"),
            [since],
            |r| read_sums(r, 0),
        ),
        None => conn.query_row(&format!("SELECT {SUM_COLS} FROM win_kimi_events"), [], |r| {
            read_sums(r, 0)
        }),
    };
    result.map_err(|e| format!("聚合 win_kimi 用量失败: {e}"))
}

/// win_kimi 按模型分组合计，按总量降序。
pub fn win_kimi_sum_by_model(conn: &Connection) -> Result<Vec<(String, UsageSums)>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT model, {SUM_COLS} FROM win_kimi_events GROUP BY model \
             ORDER BY SUM(input_tokens)+SUM(output_tokens)+SUM(cache_read)+SUM(cache_creation) DESC"
        ))
        .map_err(|e| format!("预编译 win_kimi 模型聚合失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("win_kimi 模型聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取 win_kimi 模型聚合结果失败: {e}"))
}

/// win_kimi_events 时间段内按模型分组合计。
pub fn win_kimi_sum_by_model_since(
    conn: &Connection,
    since_ms: i64,
) -> Result<Vec<(String, UsageSums)>, String> {
    sum_by_model_since_on(conn, "win_kimi_events", since_ms)
}

/// win_kimi 按天聚合的稀疏数组，近 days 天，按日期升序。
pub fn win_kimi_daily_sums(conn: &Connection, days: u32) -> Result<Vec<(String, UsageSums)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT date(ts/1000,'unixepoch','localtime') d, {SUM_COLS} \
             FROM win_kimi_events \
             WHERE ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY d ORDER BY d"
        ))
        .map_err(|e| format!("预编译 win_kimi 按天聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("win_kimi 按天聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取 win_kimi 按天聚合结果失败: {e}"))
}

/// win_kimi 按天三序列（输入/输出/缓存），近 days 天，缺口补零。
pub fn win_kimi_trend_series(conn: &Connection, days: u32) -> Result<Vec<(String, i64, i64, i64)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE days(d) AS ( \
               SELECT date('now','localtime', ?1) \
               UNION ALL \
               SELECT date(d,'+1 day') FROM days WHERE d < date('now','localtime') \
             ) \
             SELECT days.d, \
                COALESCE(SUM(u.input_tokens),0), \
                COALESCE(SUM(u.output_tokens),0), \
                COALESCE(SUM(u.cache_read),0) + COALESCE(SUM(u.cache_creation),0) \
             FROM days \
             LEFT JOIN win_kimi_events u \
               ON date(u.ts/1000,'unixepoch','localtime') = days.d \
              AND u.ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY days.d ORDER BY days.d",
        )
        .map_err(|e| format!("预编译 win_kimi 趋势聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|e| format!("win_kimi 趋势聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<(String, i64, i64, i64)>, _>>()
        .map_err(|e| format!("读取 win_kimi 趋势聚合结果失败: {e}"))
}

/// win_kimi_events 分钟速率序列，近 minutes 分钟（含当前分钟），缺口补零。
pub fn win_kimi_minute_rates(conn: &Connection, minutes: u32) -> Result<Vec<MinuteRateRow>, String> {
    minute_series_on(conn, "win_kimi_events", minutes)
}

/// win_kimi_events 在 since_ms 以来的速度聚合。
pub fn win_kimi_speed_avg(conn: &Connection, since_ms: i64) -> Result<(i64, i64), String> {
    speed_avg_on(conn, "win_kimi_events", since_ms)
}

/// win_kimi_events 最近一次带耗时的记录。
pub fn win_kimi_latest_speed(conn: &Connection) -> Result<Option<(i64, i64)>, String> {
    latest_speed_on(conn, "win_kimi_events")
}

// ---------- WSL 侧 Codex：wsl_codex_events 的写入与聚合（与 codex_events 同构，无 agent 列） ----------

/// 批量写入 wsl_codex 用量记录，单事务 + INSERT OR IGNORE 幂等，返回实际插入条数。
/// 复用 UsageEventRow，agent 字段不写入（wsl_codex_events 无此列）。
pub fn insert_wsl_codex_events(conn: &mut Connection, rows: &[UsageEventRow]) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启事务失败: {e}"))?;
    let mut inserted = 0usize;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO wsl_codex_events
                 (ts, session_id, model, input_tokens, output_tokens, cache_read, cache_creation, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|e| format!("预编译插入语句失败: {e}"))?;
        for row in rows {
            inserted += stmt
                .execute(params![
                    row.ts,
                    row.session_id,
                    row.model,
                    row.input_tokens,
                    row.output_tokens,
                    row.cache_read,
                    row.cache_creation,
                    row.duration_ms,
                ])
                .map_err(|e| format!("写入 wsl_codex_events 失败: {e}"))?;
        }
    }
    tx.commit().map_err(|e| format!("提交事务失败: {e}"))?;
    Ok(inserted)
}

/// wsl_codex_events 总记录数。
pub fn count_wsl_codex_events(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM wsl_codex_events", [], |r| r.get(0))
        .map_err(|e| format!("统计 wsl_codex_events 失败: {e}"))
}

/// wsl_codex 时间段合计；since_ms 为 None 表示全部记录。
pub fn wsl_codex_sum_usage(conn: &Connection, since_ms: Option<i64>) -> Result<UsageSums, String> {
    let result = match since_ms {
        Some(since) => conn.query_row(
            &format!("SELECT {SUM_COLS} FROM wsl_codex_events WHERE ts >= ?1"),
            [since],
            |r| read_sums(r, 0),
        ),
        None => conn.query_row(&format!("SELECT {SUM_COLS} FROM wsl_codex_events"), [], |r| {
            read_sums(r, 0)
        }),
    };
    result.map_err(|e| format!("聚合 wsl_codex 用量失败: {e}"))
}

/// wsl_codex 按模型分组合计，按总量降序。
pub fn wsl_codex_sum_by_model(conn: &Connection) -> Result<Vec<(String, UsageSums)>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT model, {SUM_COLS} FROM wsl_codex_events GROUP BY model \
             ORDER BY SUM(input_tokens)+SUM(output_tokens)+SUM(cache_read)+SUM(cache_creation) DESC"
        ))
        .map_err(|e| format!("预编译 wsl_codex 模型聚合失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("wsl_codex 模型聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取 wsl_codex 模型聚合结果失败: {e}"))
}

/// wsl_codex_events 时间段内按模型分组合计。
pub fn wsl_codex_sum_by_model_since(
    conn: &Connection,
    since_ms: i64,
) -> Result<Vec<(String, UsageSums)>, String> {
    sum_by_model_since_on(conn, "wsl_codex_events", since_ms)
}

/// wsl_codex 按天聚合的稀疏数组，近 days 天，按日期升序。
pub fn wsl_codex_daily_sums(conn: &Connection, days: u32) -> Result<Vec<(String, UsageSums)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(&format!(
            "SELECT date(ts/1000,'unixepoch','localtime') d, {SUM_COLS} \
             FROM wsl_codex_events \
             WHERE ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY d ORDER BY d"
        ))
        .map_err(|e| format!("预编译 wsl_codex 按天聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| Ok((r.get::<_, String>(0)?, read_sums(r, 1)?)))
        .map_err(|e| format!("wsl_codex 按天聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("读取 wsl_codex 按天聚合结果失败: {e}"))
}

/// wsl_codex 按天三序列（输入/输出/缓存），近 days 天，缺口补零。
pub fn wsl_codex_trend_series(conn: &Connection, days: u32) -> Result<Vec<(String, i64, i64, i64)>, String> {
    let days = days.max(1);
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE days(d) AS ( \
               SELECT date('now','localtime', ?1) \
               UNION ALL \
               SELECT date(d,'+1 day') FROM days WHERE d < date('now','localtime') \
             ) \
             SELECT days.d, \
                COALESCE(SUM(u.input_tokens),0), \
                COALESCE(SUM(u.output_tokens),0), \
                COALESCE(SUM(u.cache_read),0) + COALESCE(SUM(u.cache_creation),0) \
             FROM days \
             LEFT JOIN wsl_codex_events u \
               ON date(u.ts/1000,'unixepoch','localtime') = days.d \
              AND u.ts >= CAST(strftime('%s','now','localtime','start of day', ?1, 'utc') AS INTEGER) * 1000 \
             GROUP BY days.d ORDER BY days.d",
        )
        .map_err(|e| format!("预编译 wsl_codex 趋势聚合失败: {e}"))?;
    let offset = format!("-{} days", days - 1);
    let rows = stmt
        .query_map([offset], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|e| format!("wsl_codex 趋势聚合查询失败: {e}"))?;
    rows.collect::<Result<Vec<(String, i64, i64, i64)>, _>>()
        .map_err(|e| format!("读取 wsl_codex 趋势聚合结果失败: {e}"))
}

/// wsl_codex_events 分钟速率序列，近 minutes 分钟（含当前分钟），缺口补零。
pub fn wsl_codex_minute_rates(conn: &Connection, minutes: u32) -> Result<Vec<MinuteRateRow>, String> {
    minute_series_on(conn, "wsl_codex_events", minutes)
}

/// wsl_codex_events 在 since_ms 以来的速度聚合。
pub fn wsl_codex_speed_avg(conn: &Connection, since_ms: i64) -> Result<(i64, i64), String> {
    speed_avg_on(conn, "wsl_codex_events", since_ms)
}

/// wsl_codex_events 最近一次带耗时的记录。
pub fn wsl_codex_latest_speed(conn: &Connection) -> Result<Option<(i64, i64)>, String> {
    latest_speed_on(conn, "wsl_codex_events")
}
