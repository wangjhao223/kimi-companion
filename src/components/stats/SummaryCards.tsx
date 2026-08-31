import { formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import { cacheHitRate } from "../../lib/pricing";
import type { PeriodUsage, StatsSummary } from "../../types";

function usageTotal(p: PeriodUsage): number {
  return p.input + p.output + p.cache_read + p.cache_creation;
}

function SummaryCard({ label, usage }: { label: string; usage?: PeriodUsage }) {
  const { unit } = useUnit();
  return (
    <div className="rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <p className="text-sm text-zinc-500">{label}</p>
      <p className="mt-0.5 font-mono text-lg font-semibold text-zinc-100">
        {usage ? formatTokens(usageTotal(usage), unit) : "—"}
      </p>
      {usage && (
        <div className="mt-1.5 space-y-0 text-xs text-zinc-500">
          <p>
            输入 <span className="text-zinc-300">{formatTokens(usage.input, unit)}</span>
            {" · "}输出{" "}
            <span className="text-zinc-300">{formatTokens(usage.output, unit)}</span>
          </p>
          <p>
            缓存{" "}
            <span className="text-zinc-300">{formatTokens(usage.cache_read, unit)}</span>
          </p>
          {cacheHitRate(usage) !== null && (
            <p>
              命中率{" "}
              <span className="text-emerald-300/90">
                {((cacheHitRate(usage) ?? 0) * 100).toFixed(1)}%
              </span>
            </p>
          )}
        </div>
      )}
    </div>
  );
}

/** 四张汇总卡（今日/本周/本月/总计），2×2 网格，撑满外层网格单元。 */
export default function SummaryCards({ summary }: { summary?: StatsSummary }) {
  return (
    <div className="grid h-full grid-cols-2 grid-rows-2 gap-2.5">
      <SummaryCard label="今日" usage={summary?.today} />
      <SummaryCard label="本周" usage={summary?.this_week} />
      <SummaryCard label="本月" usage={summary?.this_month} />
      <SummaryCard label="总计" usage={summary?.total} />
    </div>
  );
}
