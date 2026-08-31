import { invoke } from "@tauri-apps/api/core";
import type {
  CodexStatus,
  HeatmapDay,
  LaunchStatus,
  LedgerStatus,
  QuotaInfo,
  StatsSummary,
  TrendPoint,
} from "../types";

export function listDistros(): Promise<string[]> {
  return invoke<string[]>("list_distros");
}

export function getLaunchStatus(distro: string | null): Promise<LaunchStatus> {
  return invoke<LaunchStatus>("get_launch_status", { distro });
}

export function startKimiWeb(distro: string): Promise<LaunchStatus> {
  return invoke<LaunchStatus>("start_kimi_web", { distro });
}

export function stopKimiWeb(): Promise<void> {
  return invoke<void>("stop_kimi_web");
}

export function checkHookInstalled(distro: string): Promise<boolean> {
  return invoke<boolean>("check_hook_installed", { distro });
}

/** 安装记账 hook。返回 true 表示顺带重启了 kimi web（让新 hook 配置生效）。 */
export function installHook(distro: string): Promise<boolean> {
  return invoke<boolean>("install_hook", { distro });
}

export function getLedgerStatus(): Promise<LedgerStatus> {
  return invoke<LedgerStatus>("get_ledger_status");
}

export function getStatsSummary(): Promise<StatsSummary> {
  return invoke<StatsSummary>("get_stats_summary");
}

export function getStatsHeatmap(days?: number): Promise<HeatmapDay[]> {
  return invoke<HeatmapDay[]>("get_stats_heatmap", { days: days ?? null });
}

export function getStatsTrend(days?: number): Promise<TrendPoint[]> {
  return invoke<TrendPoint[]>("get_stats_trend", { days: days ?? null });
}

/** 配额未知（实例未运行/请求失败）时后端返回 null，前端降级显示。 */
export function getQuota(): Promise<QuotaInfo | null> {
  return invoke<QuotaInfo | null>("get_quota");
}

// ---------- Codex 侧（Windows 本地 rollout 导入的统计） ----------

export function getCodexSummary(): Promise<StatsSummary> {
  return invoke<StatsSummary>("get_codex_summary");
}

export function getCodexHeatmap(days?: number): Promise<HeatmapDay[]> {
  return invoke<HeatmapDay[]>("get_codex_heatmap", { days: days ?? null });
}

export function getCodexTrend(days?: number): Promise<TrendPoint[]> {
  return invoke<TrendPoint[]>("get_codex_trend", { days: days ?? null });
}

export function getCodexStatus(): Promise<CodexStatus> {
  return invoke<CodexStatus>("get_codex_status");
}
