export interface LaunchStatus {
  wsl_ok: boolean;
  kimi_installed: boolean;
  running: boolean;
  port: number | null;
  url: string | null;
  remote_control: boolean;
  rc_url: string | null;
}

export interface LedgerStatus {
  total_records: number;
  last_sync_at_ms: number | null;
  last_error: string | null;
}

export interface PeriodUsage {
  input: number;
  output: number;
  cache_read: number;
  cache_creation: number;
}

export interface ModelUsage extends PeriodUsage {
  model: string;
}

export interface StatsSummary {
  today: PeriodUsage;
  this_week: PeriodUsage;
  this_month: PeriodUsage;
  total: PeriodUsage;
  by_model: ModelUsage[];
  /** 今日/本周/本月的按模型分组（费用分层显示按单价逐模型计价用） */
  today_by_model: ModelUsage[];
  this_week_by_model: ModelUsage[];
  this_month_by_model: ModelUsage[];
}

export interface HeatmapDay {
  /** 本地时区日期 YYYY-MM-DD */
  date: string;
  input: number;
  output: number;
  cache_read: number;
  cache_creation: number;
  total: number;
}

export interface TrendPoint {
  date: string;
  input: number;
  output: number;
  /** cache_read + cache_creation */
  cache: number;
}

/** 一分钟速率桶：RPM = requests，TPM = tokens（输入+输出+缓存读+缓存写合计）。 */
export interface RatePoint {
  /** 分钟桶起点（毫秒时间戳） */
  minute_start_ms: number;
  requests: number;
  tokens: number;
}

/** 单个模型的输出速度统计（tok/s）：同模型并行会话的生成速度按时间重叠叠加。
 *  仅统计 output_tokens>0 且 duration_ms>0 的有效记录。 */
export interface ModelSpeedStats {
  /** 模型名；空串表示未知模型，由前端回退显示 */
  model: string;
  /** 当前叠加速度（tok/s）：与基准记录生成区间重叠的各会话速度之和（每个会话只取其最新一条重叠记录） */
  current_speed: number;
}

/** 输出速度统计（每个模型一条，models 按基准记录 ts 降序、并列按模型名排序）。 */
export interface SpeedStats {
  models: ModelSpeedStats[];
}

/** kimi CLI 更新检查结果（check_kimi_cli_update）。 */
export interface UpdateCheckResult {
  current_version: string;
  up_to_date: boolean;
  latest_version: string | null;
  /** CLI 原始输出 */
  output: string;
}

/** kimi CLI 更新执行结果（update_kimi_cli）。 */
export interface UpdateResult {
  success: boolean;
  output: string;
  /** 更新后后端自动重启了 kimi web */
  restarted_web: boolean;
  new_version: string | null;
}

/**
 * 统计数据来源：kimi（WSL 账本）、codex（Windows 本地 rollout 导入）、
 * kimi-win（Kimi Code Windows 桌面端）或 wsl-codex（WSL 内的 codex，UNC 路径目录）。
 */
export type AgentSource = "kimi" | "codex" | "kimi-win" | "wsl-codex";

export interface CodexStatus {
  /** 是否找到 Codex 数据目录 */
  dir_found: boolean;
  /** 数据目录路径（找到时） */
  dir: string | null;
  total_records: number;
  last_sync_at_ms: number | null;
  last_error: string | null;
}

/** Kimi Code 桌面端记账同步状态，字段与 CodexStatus 一致。 */
export interface WinKimiStatus {
  /** 是否找到桌面端数据目录 */
  dir_found: boolean;
  /** 数据目录路径（找到时） */
  dir: string | null;
  total_records: number;
  last_sync_at_ms: number | null;
  last_error: string | null;
}

/** Windows 桌面程序的探测状态（当前只有 Kimi Code 桌面端一项）。 */
export interface DesktopAppInfo {
  /** 目前只有 "kimi-code" */
  id: string;
  /** 显示名："Kimi Code" */
  name: string;
  installed: boolean;
  running: boolean;
  exe_path: string | null;
}
