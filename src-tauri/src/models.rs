use serde::Serialize;
use std::sync::{Arc, Mutex};

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
    /// 今日/本周/本月的按模型分组：费用估算要按单价逐模型计价，光有跨模型
    /// 合计的 PeriodUsage 算不出各周期金额。
    pub today_by_model: Vec<ModelUsage>,
    pub this_week_by_model: Vec<ModelUsage>,
    pub this_month_by_model: Vec<ModelUsage>,
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

/// Windows 原生 Kimi Code 桌面端同步状态，字段与 CodexStatus 完全一致。
#[derive(Debug, Clone, Default, Serialize)]
pub struct WinKimiStatus {
    /// 是否找到桌面端数据目录（%USERPROFILE%\.kimi-code\sessions）
    pub dir_found: bool,
    /// 数据目录路径（找到时）
    pub dir: Option<String>,
    /// win_kimi_events 总记录数
    pub total_records: i64,
    /// 最近一次成功扫描的时间（毫秒时间戳），从未成功为 None
    pub last_sync_at_ms: Option<i64>,
    /// 最近一轮扫描中某个文件的错误信息，全部成功则为 None
    pub last_error: Option<String>,
}

/// 内存中的 win_kimi 同步状态，供 get_win_kimi_status 命令查询。
pub type SharedWinKimiStatus = Arc<Mutex<WinKimiStatus>>;

/// 一个 Windows 桌面程序（当前仅 Kimi Code）的安装与运行状态。
#[derive(Debug, Clone, Serialize)]
pub struct DesktopAppInfo {
    /// 固定标识："kimi-code"
    pub id: String,
    /// 展示名："Kimi Code"
    pub name: String,
    pub installed: bool,
    pub running: bool,
    /// 安装检测到的 exe 路径（未安装为 None）
    pub exe_path: Option<String>,
}

/// kimi CLI 更新检查结果（`timeout 90 kimi upgrade`，不带 -y）。
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub up_to_date: bool,
    pub latest_version: Option<String>,
    /// CLI 原始输出，前端折叠展示
    pub output: String,
}

/// kimi CLI 更新执行结果（`timeout 600 kimi upgrade -y`）。
#[derive(Debug, Clone, Serialize)]
pub struct UpdateResult {
    pub success: bool,
    pub output: String,
    /// 更新后是否自动重启了 kimi web
    pub restarted_web: bool,
    pub new_version: Option<String>,
}
