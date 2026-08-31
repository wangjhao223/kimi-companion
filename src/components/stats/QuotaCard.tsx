import { useQuery } from "@tanstack/react-query";
import { getQuota } from "../../lib/api";
import type { QuotaWindow } from "../../types";

const UNIT_LABELS: Record<string, string> = {
  hour: "小时",
  day: "天",
  week: "周",
  month: "个月",
};

function windowLabel(w: QuotaWindow): string {
  const unit = UNIT_LABELS[w.window_unit] ?? w.window_unit;
  return `${w.window_duration} ${unit}窗口`;
}

function QuotaBar({ w }: { w: QuotaWindow }) {
  const pct = w.limit > 0 ? Math.min(100, (w.used / w.limit) * 100) : 0;
  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-sm">
        <span className="text-zinc-300">{windowLabel(w)}</span>
        <span className="font-mono text-zinc-400">
          {w.used} / {w.limit}
        </span>
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-zinc-800">
        <div
          className={`h-full rounded-full ${pct >= 90 ? "bg-red-500" : pct >= 70 ? "bg-amber-500" : "bg-emerald-500"}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      {w.reset_at && (
        <p className="mt-1 text-xs text-zinc-600">
          重置时间：{new Date(w.reset_at).toLocaleString()}
        </p>
      )}
    </div>
  );
}

export default function QuotaCard() {
  const quotaQuery = useQuery({
    queryKey: ["quota"],
    queryFn: getQuota,
    refetchInterval: 30_000,
    retry: 1,
  });
  const quota = quotaQuery.data;
  // summary 通常也在 limits 里，去重后一起展示；summary 优先放最前
  const windows: QuotaWindow[] = (() => {
    if (!quota) return [];
    const rest = quota.limits.filter(
      (l) =>
        !quota.summary ||
        l.window_duration !== quota.summary.window_duration ||
        l.window_unit !== quota.summary.window_unit
    );
    return quota.summary ? [quota.summary, ...rest] : rest;
  })();

  return (
    <section className="h-full rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <p className="mb-2 text-sm text-zinc-400">套餐配额</p>
      {quotaQuery.isLoading ? (
        <p className="text-sm text-zinc-600">加载中…</p>
      ) : !quota ? (
        <p className="text-sm text-zinc-600">
          配额未知（需要 kimi web 运行中，且本应用已启动/接管该实例）
        </p>
      ) : windows.length === 0 ? (
        <p className="text-sm text-zinc-600">未返回配额窗口</p>
      ) : (
        <div className="space-y-2.5">
          {windows.map((w, i) => (
            <QuotaBar key={i} w={w} />
          ))}
        </div>
      )}
    </section>
  );
}
