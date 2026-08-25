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
                 (ts, session_id, agent, model, input_tokens, output_tokens, cache_read, cache_creation)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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
