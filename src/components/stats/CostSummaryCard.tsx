import {
  computeCost,
  formatCost,
  usePricing,
  type ModelPrice,
} from "../../lib/pricing";
import type { StatsSummary } from "../../types";

const EMPTY_PRICE: ModelPrice = { input: 0, output: 0, cache: 0 };

/** 费用估算小总结版：只显示合计金额，引导到详情页填写单价。 */
export default function CostSummaryCard({ summary }: { summary?: StatsSummary }) {
  const [pricing] = usePricing();
  const models = summary?.by_model ?? [];
  const totalCost = models.reduce(
    (acc, m) => acc + computeCost(m, pricing[m.model] ?? EMPTY_PRICE),
    0
  );

  return (
    <section className="rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <p className="text-sm text-zinc-500">费用估算（全部时间）</p>
      <p className="mt-0.5 font-mono text-lg font-semibold text-emerald-300">
        {models.length === 0 ? "—" : formatCost(totalCost)}
      </p>
      <p className="mt-1.5 text-xs text-zinc-500">
        按本地单价表估算，单价在「详情」页填写
      </p>
    </section>
  );
}
