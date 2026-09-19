import {
  computeCost,
  formatCost,
  usePricing,
  type ModelPrice,
} from "../../lib/pricing";
import type { ModelUsage, StatsSummary } from "../../types";

const EMPTY_PRICE: ModelPrice = { input: 0, output: 0, cache: 0, cache_write: 0 };

/** 费用估算小总结版：总金额下分层显示今日/本周/本月，单价在下方费用估算模块填写。 */
export default function CostSummaryCard({ summary }: { summary?: StatsSummary }) {
  const [pricing] = usePricing();
  // 费用按单价逐模型计价再求和：不同模型单价不同，不能用跨模型合计直接乘。
  const costOf = (list?: ModelUsage[]) =>
    (list ?? []).reduce(
      (acc, m) => acc + computeCost(m, pricing[m.model] ?? EMPTY_PRICE),
      0
    );

  const models = summary?.by_model ?? [];
  const periods: { label: string; list?: ModelUsage[] }[] = [
    { label: "今日", list: summary?.today_by_model },
    { label: "本周", list: summary?.this_week_by_model },
    { label: "本月", list: summary?.this_month_by_model },
  ];

  return (
    <section className="h-full rounded-xl border border-zinc-200 bg-white p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="text-base text-zinc-500">费用估算（全部时间）</p>
      <p className="mt-0.5 font-mono text-xl font-semibold text-emerald-600 dark:text-emerald-300">
        {models.length === 0 ? "—" : formatCost(costOf(summary?.by_model))}
      </p>
      <div className="mt-2 space-y-1 border-t border-zinc-200 pt-2 dark:border-zinc-800/60">
        {periods.map((p) => (
          <div
            key={p.label}
            className="flex items-baseline justify-between text-sm"
          >
            <span className="text-zinc-500">{p.label}</span>
            <span className="font-mono text-zinc-700 dark:text-zinc-300">
              {formatCost(costOf(p.list))}
            </span>
          </div>
        ))}
      </div>
      <p className="mt-1.5 text-sm text-zinc-500">
        按本地单价表估算，单价在下方费用模块填写
      </p>
    </section>
  );
}
