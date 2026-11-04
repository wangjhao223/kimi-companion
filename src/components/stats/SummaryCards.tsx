import { formatTokens } from "../../lib/format";
import type { Unit } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import { cacheHitRate } from "../../lib/pricing";
import {
  ProviderFilter,
  useProviderCatalog,
  useProviderSelection,
} from "../../lib/providerFilter";
import type { AgentSource, ModelUsage, PeriodUsage, StatsSummary } from "../../types";

const EMPTY_USAGE: PeriodUsage = {
  input: 0,
  output: 0,
  cache_read: 0,
  cache_creation: 0,
};

function usageTotal(p: PeriodUsage): number {
  return p.input + p.output + p.cache_read + p.cache_creation;
}

/** 0 直接显示 "0"，避免无记录周期显示成 "0M"/"0k"。 */
function formatCount(n: number, unit: Unit): string {
  return n === 0 ? "0" : formatTokens(n, unit);
}

/** 全部供应商用周期汇总；多选时按供应商过滤模型后分项求和。 */
function periodUsage(
  total: PeriodUsage | undefined,
  byModel: ModelUsage[] | undefined,
  selected: string[],
  providerOf: (model: string) => string
): PeriodUsage | undefined {
  if (selected.length === 0) return total;
  if (!byModel) return EMPTY_USAGE;
  return byModel
    .filter((m) => selected.includes(providerOf(m.model)))
    .reduce(
      (sum, usage) => ({
        input: sum.input + usage.input,
        output: sum.output + usage.output,
        cache_read: sum.cache_read + usage.cache_read,
        cache_creation: sum.cache_creation + usage.cache_creation,
      }),
      { ...EMPTY_USAGE }
    );
}

function SummaryCard({
  label,
  usage,
  providers,
  selected,
  onProvidersChange,
}: {
  label: string;
  usage?: PeriodUsage;
  providers: string[];
  selected: string[];
  onProvidersChange: (providers: string[]) => void;
}) {
  const { unit } = useUnit();
  const total = usage ? usageTotal(usage) : null;
  return (
    <div className="min-w-0 rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="text-sm text-zinc-500">{label}</p>
      <div className="mt-0.5 flex min-w-0 items-center gap-1.5">
        <p className="min-w-0 break-all font-mono text-2xl font-semibold text-zinc-900 dark:text-zinc-100">
          {total === null ? "—" : formatCount(total, unit)}
        </p>
        <ProviderFilter
          label={label}
          providers={providers}
          selected={selected}
          onChange={onProvidersChange}
        />
      </div>
      {usage && (
        <div className="mt-1 space-y-0 break-words text-base text-zinc-500">
          <p>
            输入{" "}
            <span className="text-zinc-700 dark:text-zinc-300">
              {formatCount(usage.input, unit)}
            </span>
            {" · "}输出{" "}
            <span className="text-zinc-700 dark:text-zinc-300">
              {formatCount(usage.output, unit)}
            </span>
          </p>
          <p>
            缓存{" "}
            <span className="text-zinc-700 dark:text-zinc-300">
              {formatCount(usage.cache_read, unit)}
            </span>
          </p>
          {cacheHitRate(usage) !== null && (
            <p>
              命中率{" "}
              <span className="text-emerald-600/90 dark:text-emerald-300/90">
                {((cacheHitRate(usage) ?? 0) * 100).toFixed(1)}%
              </span>
            </p>
          )}
        </div>
      )}
    </div>
  );
}

/** 四张汇总卡共享供应商多选；已删除供应商的历史用量统一归“其他”。 */
export default function SummaryCards({
  summary,
  source,
  distro,
}: {
  summary?: StatsSummary;
  source: AgentSource;
  distro?: string;
}) {
  const { providers, providerOf, ready } = useProviderCatalog(summary, source, distro);
  const [selected, setSelected] = useProviderSelection("summary", source, providers, ready);

  return (
    <div className="grid h-full grid-cols-2 grid-rows-2 gap-x-1 gap-y-2.5">
      <SummaryCard
        label="今日"
        usage={periodUsage(summary?.today, summary?.today_by_model, selected, providerOf)}
        providers={providers}
        selected={selected}
        onProvidersChange={setSelected}
      />
      <SummaryCard
        label="本周"
        usage={periodUsage(summary?.this_week, summary?.this_week_by_model, selected, providerOf)}
        providers={providers}
        selected={selected}
        onProvidersChange={setSelected}
      />
      <SummaryCard
        label="本月"
        usage={periodUsage(summary?.this_month, summary?.this_month_by_model, selected, providerOf)}
        providers={providers}
        selected={selected}
        onProvidersChange={setSelected}
      />
      <SummaryCard
        label="总计"
        usage={periodUsage(summary?.total, summary?.by_model, selected, providerOf)}
        providers={providers}
        selected={selected}
        onProvidersChange={setSelected}
      />
    </div>
  );
}
