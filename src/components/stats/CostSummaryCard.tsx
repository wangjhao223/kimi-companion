import { useQuery } from "@tanstack/react-query";
import { getCostEstimates } from "../../lib/api";
import {
  computeCost,
  formatCost,
  usePricing,
  type ModelPrice,
} from "../../lib/pricing";
import {
  ProviderFilter,
  useProviderCatalog,
  useProviderSelection,
} from "../../lib/providerFilter";
import type { AgentSource, ModelUsage, StatsSummary } from "../../types";

const EMPTY_PRICE: ModelPrice = { input: 0, output: 0, cache: 0, cache_write: 0 };

type Period = "total" | "today" | "this_week" | "this_month";

/** 费用估算小总结版：按供应商筛选，使用后端逐事件条件计价结果。 */
export default function CostSummaryCard({
  summary,
  source,
  distro,
}: {
  summary?: StatsSummary;
  source: AgentSource;
  distro?: string;
}) {
  const [pricing] = usePricing();
  const { providers, providerOf, ready } = useProviderCatalog(summary, source, distro);
  const [selected, setSelected] = useProviderSelection("cost", source, providers, ready);
  const costQuery = useQuery({
    queryKey: ["cost-estimates", source, pricing],
    queryFn: () => getCostEstimates(source, pricing),
    refetchInterval: 10_000,
    placeholderData: (previous) => previous,
  });

  const costs = new Map((costQuery.data ?? []).map((cost) => [cost.model, cost]));
  const costOf = (list: ModelUsage[] | undefined, period: Period) =>
    (list ?? [])
      .filter((m) => selected.length === 0 || selected.includes(providerOf(m.model)))
      .reduce((acc, m) => {
        const calculated = costs.get(m.model)?.[period];
        return acc +
          (calculated ?? computeCost(m, pricing[m.model] ?? EMPTY_PRICE));
      }, 0);

  const models = summary?.by_model ?? [];
  const periods: { label: string; list?: ModelUsage[]; key: Period }[] = [
    { label: "今日", list: summary?.today_by_model, key: "today" },
    { label: "本周", list: summary?.this_week_by_model, key: "this_week" },
    { label: "本月", list: summary?.this_month_by_model, key: "this_month" },
  ];

  return (
    <section className="min-w-0 h-full rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="text-sm text-zinc-500">费用估算</p>
      <div className="mt-0.5 flex min-w-0 items-center gap-1.5">
        <p className="min-w-0 break-all font-mono text-2xl font-semibold text-emerald-600 dark:text-emerald-300">
          {models.length === 0 ? "—" : formatCost(costOf(models, "total"))}
        </p>
        <ProviderFilter
          label="费用估算"
          providers={providers}
          selected={selected}
          onChange={setSelected}
        />
      </div>
      <div className="mt-1.5 space-y-0.5 border-t border-zinc-200 pt-1.5 dark:border-zinc-800/60">
        {periods.map((p) => (
          <div
            key={p.label}
            className="flex items-baseline justify-between gap-1 text-base"
          >
            <span className="shrink-0 text-zinc-500">{p.label}</span>
            <span className="min-w-0 break-all text-right font-mono text-zinc-700 dark:text-zinc-300">
              {formatCost(costOf(p.list, p.key))}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}
