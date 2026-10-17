import { invoke } from "@tauri-apps/api/core";
import type {
  AgentSource,
  CodexStatus,
  DesktopAppInfo,
  HeatmapDay,
  LaunchStatus,
  LedgerStatus,
  RatePoint,
  SpeedStats,
  StatsSummary,
  TrendPoint,
  UpdateCheckResult,
  UpdateResult,
  WinKimiStatus,
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

/** 开关 kimi web 远程控制；web 未运行时开启会顺带启动 web。 */
export function setRemoteControl(
  distro: string,
  enable: boolean
): Promise<LaunchStatus> {
  return invoke<LaunchStatus>("set_remote_control", { distro, enable });
}

/** 远程控制二维码（PNG 的 base64，无 data URL 前缀）。 */
export function getRcQrcode(distro: string): Promise<string> {
  return invoke<string>("get_rc_qrcode", { distro });
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

/** WSL 端 kimi CLI 当前版本号，如 "2.0.1"。 */
export function getKimiCliVersion(distro: string): Promise<string> {
  return invoke<string>("get_kimi_cli_version", { distro });
}

export function checkKimiCliUpdate(distro: string): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>("check_kimi_cli_update", { distro });
}

export function updateKimiCli(distro: string): Promise<UpdateResult> {
  return invoke<UpdateResult>("update_kimi_cli", { distro });
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

/** 速率：近 minutes 分钟的 RPM/TPM 分钟序列（默认 60 分钟，缺口补零）。 */
export function getStatsRates(minutes?: number): Promise<RatePoint[]> {
  return invoke<RatePoint[]>("get_stats_rates", { minutes: minutes ?? null });
}

/** 输出速度：当前瞬时速度 + 近 1 小时 / 今日加权均速（tok/s）。 */
export function getStatsSpeed(): Promise<SpeedStats> {
  return invoke<SpeedStats>("get_stats_speed");
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

export function getCodexRates(minutes?: number): Promise<RatePoint[]> {
  return invoke<RatePoint[]>("get_codex_rates", { minutes: minutes ?? null });
}

export function getCodexSpeed(): Promise<SpeedStats> {
  return invoke<SpeedStats>("get_codex_speed");
}

export function getCodexStatus(): Promise<CodexStatus> {
  return invoke<CodexStatus>("get_codex_status");
}

// ---------- Codex（WSL 侧，数据目录为 UNC 路径） ----------

export function getWslCodexSummary(): Promise<StatsSummary> {
  return invoke<StatsSummary>("get_wsl_codex_summary");
}

export function getWslCodexHeatmap(days?: number): Promise<HeatmapDay[]> {
  return invoke<HeatmapDay[]>("get_wsl_codex_heatmap", { days: days ?? null });
}

export function getWslCodexTrend(days?: number): Promise<TrendPoint[]> {
  return invoke<TrendPoint[]>("get_wsl_codex_trend", { days: days ?? null });
}

export function getWslCodexRates(minutes?: number): Promise<RatePoint[]> {
  return invoke<RatePoint[]>("get_wsl_codex_rates", { minutes: minutes ?? null });
}

export function getWslCodexSpeed(): Promise<SpeedStats> {
  return invoke<SpeedStats>("get_wsl_codex_speed");
}

export function getWslCodexStatus(): Promise<CodexStatus> {
  return invoke<CodexStatus>("get_wsl_codex_status");
}

// ---------- Kimi Code 桌面端（Windows 桌面程序的统计） ----------

export function getWinKimiSummary(): Promise<StatsSummary> {
  return invoke<StatsSummary>("get_win_kimi_summary");
}

export function getWinKimiHeatmap(days?: number): Promise<HeatmapDay[]> {
  return invoke<HeatmapDay[]>("get_win_kimi_heatmap", { days: days ?? null });
}

export function getWinKimiTrend(days?: number): Promise<TrendPoint[]> {
  return invoke<TrendPoint[]>("get_win_kimi_trend", { days: days ?? null });
}

export function getWinKimiRates(minutes?: number): Promise<RatePoint[]> {
  return invoke<RatePoint[]>("get_win_kimi_rates", { minutes: minutes ?? null });
}

export function getWinKimiSpeed(): Promise<SpeedStats> {
  return invoke<SpeedStats>("get_win_kimi_speed");
}

export function getWinKimiStatus(): Promise<WinKimiStatus> {
  return invoke<WinKimiStatus>("get_win_kimi_status");
}

// ---------- Windows 桌面程序（探测与启动） ----------

export function getDesktopAppsStatus(): Promise<DesktopAppInfo[]> {
  return invoke<DesktopAppInfo[]>("get_desktop_apps_status");
}

/** 启动桌面程序（app: "kimi-code"），失败时抛出中文错误信息。 */
export function startDesktopApp(app: string): Promise<void> {
  return invoke<void>("start_desktop_app", { app });
}

/** 数据源统计 API 收敛表：页面/组件按 AgentSource 查表取数。 */
export const STATS_APIS: Record<
  AgentSource,
  {
    summary: () => Promise<StatsSummary>;
    heatmap: (days?: number) => Promise<HeatmapDay[]>;
    trend: (days?: number) => Promise<TrendPoint[]>;
    rates: (minutes?: number) => Promise<RatePoint[]>;
    speed: () => Promise<SpeedStats>;
  }
> = {
  kimi: {
    summary: getStatsSummary,
    heatmap: getStatsHeatmap,
    trend: getStatsTrend,
    rates: getStatsRates,
    speed: getStatsSpeed,
  },
  codex: {
    summary: getCodexSummary,
    heatmap: getCodexHeatmap,
    trend: getCodexTrend,
    rates: getCodexRates,
    speed: getCodexSpeed,
  },
  "wsl-codex": {
    summary: getWslCodexSummary,
    heatmap: getWslCodexHeatmap,
    trend: getWslCodexTrend,
    rates: getWslCodexRates,
    speed: getWslCodexSpeed,
  },
  "kimi-win": {
    summary: getWinKimiSummary,
    heatmap: getWinKimiHeatmap,
    trend: getWinKimiTrend,
    rates: getWinKimiRates,
    speed: getWinKimiSpeed,
  },
};
