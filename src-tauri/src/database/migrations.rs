//! 建表迁移。当前只有 v1；全部语句幂等（IF NOT EXISTS），直接每次启动执行。

use rusqlite::Connection;

pub fn run(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS usage_events (
            id INTEGER PRIMARY KEY,
            ts INTEGER NOT NULL,              -- 毫秒时间戳
            session_id TEXT NOT NULL DEFAULT '',
            agent TEXT NOT NULL DEFAULT '',
            model TEXT NOT NULL DEFAULT '',
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read INTEGER NOT NULL DEFAULT 0,
            cache_creation INTEGER NOT NULL DEFAULT 0,
            -- 幂等去重：文本列取 NOT NULL DEFAULT ''，否则 NULL 不参与 UNIQUE 判等
            UNIQUE(ts, session_id, model, input_tokens, output_tokens)
        );
        CREATE INDEX IF NOT EXISTS idx_usage_day ON usage_events(ts);

        -- Codex 侧（Windows 本地 rollout 文件导入）。与 usage_events 同构，
        -- 去掉 agent 列（agent 在 kimi 侧是会话内子代理 id，并非产品标识）。
        -- 增量游标按文件存 meta["codex_off:<path>"]，不单独建表。
        CREATE TABLE IF NOT EXISTS codex_events (
            id INTEGER PRIMARY KEY,
            ts INTEGER NOT NULL,              -- 毫秒时间戳
            session_id TEXT NOT NULL DEFAULT '',
            model TEXT NOT NULL DEFAULT '',
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read INTEGER NOT NULL DEFAULT 0,
            cache_creation INTEGER NOT NULL DEFAULT 0,
            UNIQUE(ts, session_id, model, input_tokens, output_tokens)
        );
        CREATE INDEX IF NOT EXISTS idx_codex_day ON codex_events(ts);

        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("数据库迁移失败: {e}"))
}
