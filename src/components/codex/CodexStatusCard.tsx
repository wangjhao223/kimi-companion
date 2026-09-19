import { useQuery } from "@tanstack/react-query";
import { getCodexStatus, getWslCodexStatus } from "../../lib/api";
import type { CodexStatus } from "../../types";

/** Codex 数据来源：codex = Windows 本地；wsl-codex = WSL 内（目录为 UNC 路径）。 */
export type CodexSource = "codex" | "wsl-codex";

/** 按来源查表取数（与 TrendChart 的 STATS_APIS 同一模式）。queryKey 与所在页面的探测查询同名，缓存共享。 */
const SOURCE_CONFIG: Record<
  CodexSource,
  {
    queryKey: string;
    queryFn: () => Promise<CodexStatus>;
    title: string;
    /** 「数据目录」未找到时的提示 */
    dirMissing: string;
  }
> = {
  codex: {
    queryKey: "codex-status",
    queryFn: getCodexStatus,
    title: "Codex 数据",
    dirMissing: "未找到 ~/.codex（Codex 未安装或尚未使用过）",
  },
  "wsl-codex": {
    queryKey: "wsl-codex-status",
    queryFn: getWslCodexStatus,
    title: "Codex 数据（WSL）",
    dirMissing: "未找到 WSL 内的 ~/.codex（Codex 未安装或尚未使用过）",
  },
};

/** Codex 数据同步状态卡：数据目录 / 记录数 / 最近同步。后台线程 15s 一轮扫描。 */
export default function CodexStatusCard({ source }: { source: CodexSource }) {
  const config = SOURCE_CONFIG[source];
  const statusQuery = useQuery({
    queryKey: [config.queryKey],
    queryFn: config.queryFn,
    refetchInterval: 10_000,
  });
  const status = statusQuery.data;

  return (
    <section className="h-full rounded-xl border border-zinc-200 bg-white p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="text-base text-zinc-500 dark:text-zinc-400">{config.title}</p>
      <div className="mt-2 space-y-2">
        <div>
          <p className="text-sm text-zinc-500">数据目录</p>
          <p
            className="truncate font-mono text-base text-zinc-700 dark:text-zinc-300"
            title={status?.dir ?? undefined}
          >
            {status
              ? status.dir_found
                ? status.dir
                : config.dirMissing
              : "—"}
          </p>
        </div>
        <div className="flex items-center justify-between text-base">
          <span className="text-zinc-500">已导入记录</span>
          <span className="font-mono text-zinc-900 dark:text-zinc-100">
            {status ? status.total_records : "—"} 条
          </span>
        </div>
        <div className="flex items-center justify-between text-base">
          <span className="text-zinc-500">最近同步</span>
          <span className="text-zinc-700 dark:text-zinc-300">
            {status?.last_sync_at_ms
              ? new Date(status.last_sync_at_ms).toLocaleTimeString()
              : "尚未同步"}
          </span>
        </div>
      </div>
      {/* 目录未找到的提示已在「数据目录」行展示，这里只显示文件级错误 */}
      {status?.dir_found && status.last_error && (
        <p className="mt-2 rounded-md border border-red-300 bg-red-50 px-3 py-2 text-base text-red-600 dark:border-red-900/60 dark:bg-red-950/40 dark:text-red-300">
          同步出错:{status.last_error}
        </p>
      )}
    </section>
  );
}
