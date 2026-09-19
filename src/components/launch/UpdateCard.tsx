import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  checkKimiCliUpdate,
  getKimiCliVersion,
  listDistros,
  updateKimiCli,
} from "../../lib/api";

/** 取多行输出的尾部几行，用于更新失败时的错误摘要。 */
function tailLines(text: string, n = 5): string {
  const lines = text
    .split("\n")
    .map((l) => l.trimEnd())
    .filter((l) => l.trim() !== "");
  return lines.slice(-n).join("\n");
}

/** 状态点：样式仿 LaunchPanel 的 StatusDot，多一个红色错误态。 */
function StatusDot({
  tone,
  busy,
}: {
  tone: "ok" | "warn" | "err" | "idle";
  busy?: boolean;
}) {
  const color = busy
    ? "bg-amber-400 animate-pulse"
    : tone === "ok"
      ? "bg-emerald-400"
      : tone === "warn"
        ? "bg-amber-400"
        : tone === "err"
          ? "bg-red-500"
          : "bg-zinc-600";
  return <span className={`inline-block h-2.5 w-2.5 rounded-full ${color}`} />;
}

/**
 * Kimi Code CLI 更新卡：显示 WSL 端 kimi CLI 当前版本，挂载时自动检查更新，
 * 支持一键更新（下载约 178MB）。distro 与 LaunchPanel 保持一致——LaunchPanel 的
 * 发行版选择是组件内 state（未持久化），默认取 listDistros 的第一个；这里用同一个
 * ["distros"] 查询（缓存共享）并取相同的默认值。
 */
export default function UpdateCard() {
  const queryClient = useQueryClient();

  const distrosQuery = useQuery({
    queryKey: ["distros"],
    queryFn: listDistros,
    retry: 1,
  });
  const distro = distrosQuery.data?.[0] ?? null;

  const versionQuery = useQuery({
    queryKey: ["kimi-cli-version", distro],
    queryFn: () => getKimiCliVersion(distro!),
    enabled: distro !== null,
  });

  // 挂载即自动检查一次；失败后由「重试」按钮手动 refetch
  const checkQuery = useQuery({
    queryKey: ["kimi-cli-update-check", distro],
    queryFn: () => checkKimiCliUpdate(distro!),
    enabled: distro !== null,
    retry: false,
    refetchOnWindowFocus: false,
  });

  const updateMutation = useMutation({
    mutationFn: (d: string) => updateKimiCli(d),
    onSuccess: (result) => {
      if (result.success) {
        // 更新成功后重新拿当前版本，并让检查状态随之刷新为「已是最新」
        queryClient.invalidateQueries({ queryKey: ["kimi-cli-version"] });
        queryClient.invalidateQueries({ queryKey: ["kimi-cli-update-check"] });
      }
    },
  });

  const check = checkQuery.data;
  const update = updateMutation.data;
  const updating = updateMutation.isPending;

  const dotTone: "ok" | "warn" | "err" | "idle" =
    update || updateMutation.isError
      ? update?.success
        ? "ok"
        : "err"
      : checkQuery.isError
        ? "err"
        : check
          ? check.up_to_date
            ? "ok"
            : "warn"
          : "idle";
  const dotBusy = updating || (distro !== null && checkQuery.isPending);

  const output = update?.output ?? check?.output ?? "";

  return (
    <section className="h-full rounded-xl border border-zinc-200 bg-white p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <div className="flex items-center justify-between">
        <p className="text-base text-zinc-500 dark:text-zinc-400">Kimi Code 更新</p>
        <StatusDot tone={dotTone} busy={dotBusy} />
      </div>
      <p className="mt-1.5 text-base text-zinc-500">
        当前版本{" "}
        <span className="font-mono text-zinc-900 dark:text-zinc-100">
          {!distro
            ? "—"
            : versionQuery.data
              ? `v${versionQuery.data}`
              : versionQuery.isPending
                ? "…"
                : "未知"}
        </span>
      </p>

      <div className="mt-1.5">
        {!distro ? (
          <p className="text-base text-zinc-400 dark:text-zinc-600">未检测到 WSL 发行版</p>
        ) : updating ? (
          <p className="text-sm text-zinc-500">
            下载约 178MB，可能需要几分钟，请勿关闭窗口
          </p>
        ) : update ? (
          update.success ? (
            <div>
              <p className="text-base text-emerald-600 dark:text-emerald-300">
                已更新到 v{update.new_version ?? versionQuery.data ?? "?"}
              </p>
              {update.restarted_web && (
                <p className="mt-0.5 text-sm text-zinc-500">
                  kimi web 已自动重启
                </p>
              )}
            </div>
          ) : (
            <p className="whitespace-pre-wrap break-all text-base text-red-600 dark:text-red-300">
              更新失败：{tailLines(update.output) || "未知错误"}
            </p>
          )
        ) : updateMutation.isError ? (
          <p className="text-base text-red-600 dark:text-red-300">
            更新失败：{(updateMutation.error as Error).message}
          </p>
        ) : checkQuery.isPending ? (
          <p className="flex items-center gap-2 text-base text-zinc-500">
            <span className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-zinc-300 border-t-zinc-600 dark:border-zinc-600 dark:border-t-zinc-300" />
            正在检查更新…
          </p>
        ) : checkQuery.isError ? (
          <p className="text-base text-red-600 dark:text-red-300">
            检查失败：{(checkQuery.error as Error).message}
          </p>
        ) : check?.up_to_date ? (
          <div className="flex items-center justify-between">
            <p className="text-base text-emerald-600 dark:text-emerald-300">已是最新版本</p>
            <button
              className="text-sm text-zinc-500 hover:text-zinc-700 disabled:opacity-50 dark:hover:text-zinc-300"
              onClick={() => checkQuery.refetch()}
              disabled={checkQuery.isFetching}
            >
              重新检查
            </button>
          </div>
        ) : check ? (
          <p className="text-base text-amber-600 dark:text-amber-300">
            发现新版本 v{check.latest_version ?? "?"}
          </p>
        ) : null}
      </div>

      <div className="mt-2 flex gap-2">
        {distro &&
          !update &&
          !updateMutation.isError &&
          check &&
          !check.up_to_date && (
            <button
              className="flex-1 rounded-lg bg-emerald-600 px-3 py-1.5 text-base font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
              onClick={() => updateMutation.mutate(distro)}
              disabled={updating}
            >
              {updating ? (
                <span className="flex items-center justify-center gap-2">
                  <span className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/40 border-t-white" />
                  正在更新…
                </span>
              ) : (
                "立即更新"
              )}
            </button>
          )}
        {checkQuery.isError && !update && !updateMutation.isError && (
          <button
            className="flex-1 rounded-lg border border-zinc-300 px-3 py-1.5 text-base hover:bg-zinc-200 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800"
            onClick={() => checkQuery.refetch()}
            disabled={checkQuery.isFetching}
          >
            重试
          </button>
        )}
        {!updating && (updateMutation.isError || (update && !update.success)) && (
          <button
            className="flex-1 rounded-lg border border-zinc-300 px-3 py-1.5 text-base hover:bg-zinc-200 dark:border-zinc-700 dark:hover:bg-zinc-800"
            onClick={() => updateMutation.mutate(distro!)}
          >
            重试更新
          </button>
        )}
        {update?.success && (
          <button
            className="flex-1 rounded-lg border border-zinc-300 px-3 py-1.5 text-base hover:bg-zinc-200 dark:border-zinc-700 dark:hover:bg-zinc-800"
            onClick={() => updateMutation.reset()}
          >
            重新检查
          </button>
        )}
      </div>

      {output.trim() !== "" && (
        <details className="mt-2">
          <summary className="cursor-pointer select-none text-sm text-zinc-400 hover:text-zinc-500 dark:text-zinc-600 dark:hover:text-zinc-400">
            CLI 原始输出
          </summary>
          <pre className="mt-1 max-h-24 overflow-auto whitespace-pre-wrap break-all rounded-md bg-zinc-100 p-2 text-xs leading-snug text-zinc-500 dark:bg-zinc-950/60">
            {output}
          </pre>
        </details>
      )}
    </section>
  );
}
