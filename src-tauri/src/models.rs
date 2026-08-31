use serde::Serialize;

/// 当前正在运行（或已被本应用接管）的 kimi web 实例。
#[derive(Debug, Clone, Serialize)]
pub struct RunningInstance {
    pub distro: String,
    pub port: u16,
    pub token: String,
}

/// 返回给前端的启动页整体状态。
#[derive(Debug, Clone, Serialize)]
pub struct LaunchStatus {
    pub wsl_ok: bool,
    pub kimi_installed: bool,
    pub running: bool,
    /// 是否有 kimi TUI（交互会话）在运行
    pub tui_running: bool,
    pub port: Option<u16>,
    /// 完整可打开的地址，含 token fragment：`http://127.0.0.1:<port>/#token=<token>`
    pub url: Option<String>,
}

/// 一个时间段内的 token 用量合计。
#[derive(Debug, Clone, Default, Serialize)]
pub struct PeriodUsage {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
}

/// 按模型分组的用量合计。
#[derive(Debug, Clone, Serialize)]
pub struct ModelUsage {
    pub model: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
}

/// 仪表盘汇总：今日/本周/本月/总计（本地时区边界，周一为一周开始）+ 按模型分组。
#[derive(Debug, Clone, Serialize)]
pub struct StatsSummary {
    pub today: PeriodUsage,
    pub this_week: PeriodUsage,
    pub this_month: PeriodUsage,
    pub total: PeriodUsage,
    pub by_model: Vec<ModelUsage>,
}

/// 热力图的一天（稀疏数组，只包含有数据的天）。
#[derive(Debug, Clone, Serialize)]
pub struct HeatmapDay {
    /// 本地时区日期，YYYY-MM-DD
    pub date: String,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub total: i64,
}

/// 趋势图的一点（缺口已补零）。cache = cache_read + cache_creation。
#[derive(Debug, Clone, Serialize)]
pub struct TrendPoint {
    pub date: String,
    pub input: i64,
    pub output: i64,
    pub cache: i64,
}

/// 套餐配额的一个窗口（宽松解析自 kimi web 的 /api/v1/oauth/usage）。
#[derive(Debug, Clone, Serialize)]
pub struct QuotaWindow {
    pub window_duration: i64,
    pub window_unit: String,
    pub used: f64,
    pub limit: f64,
    pub reset_at: Option<String>,
}

/// 配额信息：summary 主窗口 + limits 全部窗口。
#[derive(Debug, Clone, Serialize)]
pub struct QuotaInfo {
    pub summary: Option<QuotaWindow>,
    pub limits: Vec<QuotaWindow>,
}

/// 账本同步状态（内存快照 + 实时总数），供「账本同步」展示。
#[derive(Debug, Clone, Default, Serialize)]
pub struct LedgerStatus {
    /// usage_events 总记录数
    pub total_records: i64,
    /// 最近一次成功同步的时间（毫秒时间戳），从未成功为 None
    pub last_sync_at_ms: Option<i64>,
    /// 最近一轮同步的错误信息，成功则为 None
    pub last_error: Option<String>,
}

/// Codex 侧同步状态（内存快照 + 实时总数），供 codex 页状态卡展示。
#[derive(Debug, Clone, Default, Serialize)]
pub struct CodexStatus {
    /// 是否找到 Codex 数据目录（%USERPROFILE%\.codex）
    pub dir_found: bool,
    /// 数据目录路径（找到时）
    pub dir: Option<String>,
    /// codex_events 总记录数
    pub total_records: i64,
    /// 最近一次成功扫描的时间（毫秒时间戳），从未成功为 None
    pub last_sync_at_ms: Option<i64>,
    /// 最近一轮扫描中某个文件的错误信息，全部成功则为 None
    pub last_error: Option<String>,
}
