export interface LaunchStatus {
  wsl_ok: boolean;
  kimi_installed: boolean;
  running: boolean;
  tui_running: boolean;
  port: number | null;
  url: string | null;
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

export interface QuotaWindow {
  window_duration: number;
  window_unit: string;
  used: number;
  limit: number;
  reset_at: string | null;
}

export interface QuotaInfo {
  summary: QuotaWindow | null;
  limits: QuotaWindow[];
}

/** 统计数据来源：kimi（WSL 账本）或 codex（Windows 本地 rollout 导入）。 */
export type AgentSource = "kimi" | "codex";

export interface CodexStatus {
  /** 是否找到 Codex 数据目录 */
  dir_found: boolean;
  /** 数据目录路径（找到时） */
  dir: string | null;
  total_records: number;
  last_sync_at_ms: number | null;
  last_error: string | null;
}
