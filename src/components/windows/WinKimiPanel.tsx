import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getDesktopAppsStatus,
  getWinKimiStatus,
  startDesktopApp,
} from "../../lib/api";

// 灯样式仿写自 LaunchPanel（其内部逻辑不动，故不复用导出）
function StatusDot({ ok, busy }: { ok: boolean; busy?: boolean }) {
  const color = busy
    ? "bg-amber-400 animate-pulse"
    : ok
      ? "bg-emerald-400"
      : "bg-zinc-600";
  return <span className={`inline-block h-2.5 w-2.5 rounded-full ${color}`} />;
}

function StatusRow({
  label,
  ok,
  busy,
  detail,
}: {
  label: string;
  ok: boolean;
  busy?: boolean;
  detail?: string;
}) {
  return (
    <div className="flex items-center justify-between py-0.5">
      <div className="flex items-center gap-3">
        <StatusDot ok={ok} busy={busy} />
        <span className="text-base text-zinc-700 dark:text-zinc-300">{label}</span>
      </div>
      {detail && <span className="text-base text-zinc-500">{detail}</span>}
    </div>
  );
}

/** Kimi Code 桌面端面板：安装/运行状态灯、记账同步状态、启动按钮。 */
export default function WinKimiPanel() {
  const queryClient = useQueryClient();

  // 桌面程序探测（后端只返回 kimi-code 一项），5s 一轮
  const appsQuery = useQuery({
    queryKey: ["desktop-apps-status"],
    queryFn: getDesktopAppsStatus,
    refetchInterval: 5_000,
  });
  const app = appsQuery.data?.find((a) => a.id === "kimi-code");

  const statusQuery = useQuery({
    queryKey: ["win-kimi-status"],
    queryFn: getWinKimiStatus,
    refetchInterval: 10_000,
  });
  const status = statusQuery.data;

  const startMutation = useMutation({
    mutationFn: () => startDesktopApp("kimi-code"),
    onSettled: () => {
      // 启动后运行状态会翻转，刷新探测结果
      queryClient.invalidateQueries({ queryKey: ["desktop-apps-status"] });
    },
  });

  const installed = app?.installed ?? false;
  const error =
    (startMutation.error as Error | null)?.message ??
    (appsQuery.error as Error | null)?.message ??
    null;

  return (
    <section className="flex h-full flex-col rounded-xl border border-zinc-200 bg-white p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="text-base text-zinc-500 dark:text-zinc-400">Kimi Code 桌面端</p>
      {app?.exe_path && (
        <p
          className="mt-0.5 truncate font-mono text-sm text-zinc-400 dark:text-zinc-600"
          title={app.exe_path}
        >
          {app.exe_path}
        </p>
      )}

      <div className="mt-2 divide-y divide-zinc-300 dark:divide-zinc-800">
        <StatusRow label="已安装" ok={installed} />
        <StatusRow
          label="运行中"
          ok={app?.running ?? false}
          busy={startMutation.isPending}
        />
      </div>

      {/* 记账同步状态（展示方式参照 CodexStatusCard） */}
      <div className="mt-2.5">
        <p className="text-sm text-zinc-500">记账同步</p>
        <div className="mt-1 space-y-1.5">
          <div>
            <p
              className="truncate font-mono text-base text-zinc-700 dark:text-zinc-300"
              title={status?.dir ?? undefined}
            >
              {status
                ? status.dir_found
                  ? status.dir
                  : "未找到数据目录（桌面端尚未使用过）"
                : "—"}
            </p>
          </div>
          <div className="flex items-center justify-between text-base">
            <span className="text-zinc-500">已入库事件</span>
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
        {status?.dir_found && status.last_error && (
          <p className="mt-2 rounded-md border border-red-300 bg-red-50 px-3 py-2 text-base text-red-600 dark:border-red-900/60 dark:bg-red-950/40 dark:text-red-300">
            同步出错:{status.last_error}
          </p>
        )}
      </div>

      <div className="mt-auto pt-2.5">
        <button
          className="w-full rounded-lg bg-emerald-600 px-4 py-2 text-base font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
          onClick={() => startMutation.mutate()}
          disabled={startMutation.isPending || !installed}
          title={installed ? undefined : "未安装 Kimi Code 桌面端"}
        >
          {startMutation.isPending ? "正在启动…" : "启动"}
        </button>
        {!installed && (
          <p className="mt-1.5 text-sm text-zinc-500">
            未检测到 Kimi Code 桌面端，安装后才能启动
          </p>
        )}
        {error && (
          <p className="mt-2 rounded-md border border-red-300 bg-red-50 px-3 py-2 text-base text-red-600 dark:border-red-900/60 dark:bg-red-950/40 dark:text-red-300">
            {error}
          </p>
        )}
      </div>
    </section>
  );
}
