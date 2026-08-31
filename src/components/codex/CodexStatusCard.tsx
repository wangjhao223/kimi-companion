import { useQuery } from "@tanstack/react-query";
import { getCodexStatus } from "../../lib/api";

/** Codex 数据同步状态卡：数据目录 / 记录数 / 最近同步。后台线程 15s 一轮扫描。 */
export default function CodexStatusCard() {
  const statusQuery = useQuery({
    queryKey: ["codex-status"],
    queryFn: getCodexStatus,
    refetchInterval: 10_000,
  });
  const status = statusQuery.data;

  return (
    <section className="h-full rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <p className="text-sm text-zinc-400">Codex 数据</p>
      <div className="mt-2 space-y-2">
        <div>
          <p className="text-xs text-zinc-500">数据目录</p>
          <p
            className="truncate font-mono text-[13px] text-zinc-300"
            title={status?.dir ?? undefined}
          >
            {status
              ? status.dir_found
                ? status.dir
                : "未找到 ~/.codex（Codex 未安装或尚未使用过）"
              : "—"}
          </p>
        </div>
        <div className="flex items-center justify-between text-sm">
          <span className="text-zinc-500">已导入记录</span>
          <span className="font-mono text-zinc-100">
            {status ? status.total_records : "—"} 条
          </span>
        </div>
        <div className="flex items-center justify-between text-sm">
          <span className="text-zinc-500">最近同步</span>
          <span className="text-zinc-300">
            {status?.last_sync_at_ms
              ? new Date(status.last_sync_at_ms).toLocaleTimeString()
              : "尚未同步"}
          </span>
        </div>
      </div>
      {/* 目录未找到的提示已在「数据目录」行展示，这里只显示文件级错误 */}
      {status?.dir_found && status.last_error && (
        <p className="mt-2 rounded-md border border-red-900/60 bg-red-950/40 px-3 py-2 text-sm text-red-300">
          同步出错:{status.last_error}
        </p>
      )}
    </section>
  );
}
