import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  checkHookInstalled,
  getLaunchStatus,
  installHook,
  listDistros,
  startKimiWeb,
  stopKimiWeb,
} from "../../lib/api";

const STARTING_HINTS = [
  "正在唤醒 WSL…",
  "正在启动 kimi web…",
  "等待健康检查…",
  "读取访问 token…",
];

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
        <span className="text-sm text-zinc-300">{label}</span>
      </div>
      {detail && <span className="text-sm text-zinc-500">{detail}</span>}
    </div>
  );
}

/** 启动控制面板：发行版选择、状态灯、启停按钮、地址区、hook 安装横幅。 */
export default function LaunchPanel() {
  const queryClient = useQueryClient();
  const [distro, setDistro] = useState<string | null>(null);
  const [hintIdx, setHintIdx] = useState(0);

  const distrosQuery = useQuery({
    queryKey: ["distros"],
    queryFn: listDistros,
    retry: 1,
    // wsl.exe 偶发失败（VM 未启动等）时不能一次定终身：失败期间每 3 秒自动重试，
    // 拿到列表后停止轮询
    refetchInterval: (query) => (query.state.data ? false : 3000),
  });

  // 默认选第一个发行版（M2 设置页再做持久化选择）
  useEffect(() => {
    if (!distro && distrosQuery.data && distrosQuery.data.length > 0) {
      setDistro(distrosQuery.data[0]);
    }
  }, [distro, distrosQuery.data]);

  const statusQuery = useQuery({
    queryKey: ["launch-status", distro],
    queryFn: () => getLaunchStatus(distro),
    refetchInterval: 2500,
    enabled: distro !== null,
  });

  // 记账 hook 是否已安装（跟随发行版切换重新检测）
  const hookQuery = useQuery({
    queryKey: ["hook-installed", distro],
    queryFn: () => checkHookInstalled(distro!),
    enabled: distro !== null,
  });

  const installMutation = useMutation({
    mutationFn: (d: string) => installHook(d),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["hook-installed"] });
      queryClient.invalidateQueries({ queryKey: ["ledger-status"] });
      // 安装可能顺带重启了 kimi web（端口/URL 会变），刷新启动状态
      queryClient.invalidateQueries({ queryKey: ["launch-status"] });
    },
  });

  const startMutation = useMutation({
    mutationFn: (d: string) => startKimiWeb(d),
    onSuccess: (data) => {
      // 启动成功后直接打开 web 界面——「一键启动」的终点是页面可见
      if (data.url) {
        openUrl(data.url).catch(() => {});
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["launch-status"] });
    },
  });

  const stopMutation = useMutation({
    mutationFn: stopKimiWeb,
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["launch-status"] });
    },
  });

  const starting = startMutation.isPending;

  // 启动期间轮播进度文案
  useEffect(() => {
    if (!starting) return;
    const timer = setInterval(
      () => setHintIdx((i) => (i + 1) % STARTING_HINTS.length),
      2500
    );
    return () => clearInterval(timer);
  }, [starting]);

  const status = statusQuery.data;
  const error =
    (startMutation.error as Error | null)?.message ??
    (stopMutation.error as Error | null)?.message ??
    (statusQuery.error as Error | null)?.message ??
    null;
  const installError = (installMutation.error as Error | null)?.message ?? null;
  const hookInstalled = hookQuery.data === true;

  return (
    <div className="flex h-full flex-col gap-2">
      {hookQuery.data === false && (
        <section className="rounded-xl border border-amber-900/60 bg-amber-950/40 p-3">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-sm font-medium text-amber-200">
                记账 hook 未安装
              </p>
              <p className="mt-0.5 text-sm text-amber-300/70">
                安装后 Kimi Companion 才能记录 token 用量（写入 WSL 的
                ~/.kimi-code，含自动备份）
              </p>
            </div>
            <button
              className="shrink-0 rounded-lg bg-amber-600 px-3.5 py-1.5 text-sm font-medium text-white hover:bg-amber-500 disabled:opacity-50"
              onClick={() => distro && installMutation.mutate(distro)}
              disabled={installMutation.isPending || !distro}
            >
              {installMutation.isPending ? (
                <span className="flex items-center gap-2">
                  <span className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/40 border-t-white" />
                  安装中…
                </span>
              ) : (
                "一键安装记账"
              )}
            </button>
          </div>
          {installMutation.isSuccess && !installError && (
            <p className="mt-1.5 text-sm text-emerald-300">
              {installMutation.data
                ? "安装完成，历史数据已回填；已自动重启 kimi web 使记账立即生效"
                : "安装完成，历史数据已回填"}
            </p>
          )}
          {installError && (
            <p className="mt-1.5 text-sm text-red-300">{installError}</p>
          )}
        </section>
      )}

      <section className="flex-1 rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
        <div className="mb-1.5 flex items-center justify-between">
          <label className="text-sm text-zinc-400">WSL 发行版</label>
          <select
            className="rounded-md border border-zinc-700 bg-zinc-800 px-2 py-1 text-sm outline-none focus:border-zinc-500"
            value={distro ?? ""}
            onChange={(e) => setDistro(e.target.value || null)}
            disabled={!distrosQuery.data?.length}
          >
            {distrosQuery.data?.length ? null : (
              <option value="">未检测到发行版</option>
            )}
            {distrosQuery.data?.map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        </div>
        {distrosQuery.isError && (
          <p className="mb-1.5 rounded-md border border-red-900/60 bg-red-950/40 px-3 py-2 text-sm text-red-300">
            获取 WSL 发行版失败：
            {(distrosQuery.error as Error | null)?.message ?? "未知错误"}
            （每 3 秒自动重试）
          </p>
        )}

        <div className="divide-y divide-zinc-800">
          <StatusRow
            label="WSL 状态"
            ok={status?.wsl_ok ?? false}
            detail={distro ?? undefined}
          />
          <StatusRow
            label="kimi 已安装"
            ok={status?.kimi_installed ?? false}
          />
          <StatusRow
            label="kimi web 运行中"
            ok={status?.running ?? false}
            busy={starting}
            detail={status?.port ? `端口 ${status.port}` : undefined}
          />
          <StatusRow
            label="Kimi Code CLI 运行中"
            ok={status?.tui_running ?? false}
          />
          <StatusRow
            label="记账 hook"
            ok={hookInstalled}
            busy={hookQuery.isFetching && hookQuery.data === undefined}
          />
        </div>

        <div className="mt-2.5 flex gap-2">
          {status?.running ? (
            <button
              className="flex-1 rounded-lg bg-red-600/90 px-4 py-2 text-sm font-medium hover:bg-red-500 disabled:opacity-50"
              onClick={() => stopMutation.mutate()}
              disabled={stopMutation.isPending}
            >
              {stopMutation.isPending ? "正在停止…" : "停止 kimi（web + CLI）"}
            </button>
          ) : (
            <button
              className="flex-1 rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium hover:bg-emerald-500 disabled:opacity-50"
              onClick={() => distro && startMutation.mutate(distro)}
              disabled={starting || !distro}
            >
              {starting ? STARTING_HINTS[hintIdx] : "启动 kimi web"}
            </button>
          )}
        </div>

        {starting && (
          <p className="mt-2 flex items-center gap-2 text-sm text-amber-300/90">
            <span className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-amber-300/40 border-t-amber-300" />
            首次启动需要冷启动 WSL，请耐心等待（最长约 30 秒）
          </p>
        )}

        {error && (
          <p className="mt-2 rounded-md border border-red-900/60 bg-red-950/40 px-3 py-2 text-sm text-red-300">
            {error}
          </p>
        )}
      </section>

      {status?.running && status.url && (
        <section className="rounded-xl border border-zinc-800/60 bg-zinc-900/50 px-3 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
          <div className="flex items-center gap-2.5">
            <div className="min-w-0 flex-1">
              <p className="text-xs text-zinc-500">kimi web 地址</p>
              <p
                className="truncate font-mono text-[13px] text-emerald-300"
                title={status.url}
              >
                {status.url}
              </p>
            </div>
            <button
              className="shrink-0 rounded-lg border border-zinc-700 px-3.5 py-1.5 text-sm hover:bg-zinc-800"
              onClick={() => status.url && openUrl(status.url)}
            >
              在浏览器打开
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
