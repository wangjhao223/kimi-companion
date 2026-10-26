import { useEffect, useState } from "react";
import { abbrevModel, formatTokens } from "../../lib/format";
import type { Unit } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import { cacheHitRate } from "../../lib/pricing";
import type { AgentSource, ModelUsage, PeriodUsage, StatsSummary } from "../../types";

const ALL_MODELS = "__all__";

function summaryModelKey(source: AgentSource): string {
  return `kimi-companion.summary-model.${source}`;
}

/** 读取该数据源保存的模型筛选；存储不可用时回退全部模型。 */
function readSelectedModel(source: AgentSource): string {
  try {
    return localStorage.getItem(summaryModelKey(source)) ?? ALL_MODELS;
  } catch {
    return ALL_MODELS;
  }
}

/** 写回该数据源的模型筛选；写入失败只保留内存选择。 */
function persistSelectedModel(source: AgentSource, model: string): void {
  try {
    localStorage.setItem(summaryModelKey(source), model);
  } catch {
    // 存储不可用时仅保留内存状态。
  }
}

/** summary 尚未加载时视为可用，避免提前清除已保存的选择。 */
function isModelAvailable(summary: StatsSummary | undefined, model: string): boolean {
  if (model === ALL_MODELS || !summary) return true;
  return summary.by_model.some((m) => m.model === model);
}

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

/** 全部模型用周期汇总；指定模型无记录时回退全 0。 */
function periodUsage(
  total: PeriodUsage | undefined,
  byModel: ModelUsage[] | undefined,
  model: string
): PeriodUsage | undefined {
  if (model === ALL_MODELS) return total;
  return byModel?.find((m) => m.model === model) ?? EMPTY_USAGE;
}

function ModelSelect({
  label,
  models,
  value,
  onChange,
}: {
  label: string;
  models: ModelUsage[];
  value: string;
  onChange: (model: string) => void;
}) {
  const raw = models.find((m) => m.model === value)?.model ?? value;
  return (
    <select
      aria-label={`${label}模型筛选`}
      title={value === ALL_MODELS ? "全部模型" : raw.trim() || "未知模型"}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-20 shrink-0 cursor-pointer rounded-md border border-zinc-300 bg-white px-1 py-0.5 text-xs text-zinc-600 outline-none transition-colors focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-300 dark:focus:border-zinc-500"
    >
      <option value={ALL_MODELS}>全部模型</option>
      {models.map((m) => (
        <option key={m.model} value={m.model}>
          {m.model.trim() ? abbrevModel(m.model) : "未知模型"}
        </option>
      ))}
    </select>
  );
}

function SummaryCard({
  label,
  usage,
  models,
  selectedModel,
  onModelChange,
}: {
  label: string;
  usage?: PeriodUsage;
  models: ModelUsage[];
  selectedModel: string;
  onModelChange: (model: string) => void;
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
        <ModelSelect
          label={label}
          models={models}
          value={selectedModel}
          onChange={onModelChange}
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

/** 四张汇总卡（今日/本周/本月/总计），2×2 网格；共享同一个模型筛选并按数据源持久化。 */
export default function SummaryCards({
  summary,
  source,
}: {
  summary?: StatsSummary;
  source: AgentSource;
}) {
  const [selectedModel, setSelectedModel] = useState(() => readSelectedModel(source));
  const models = summary?.by_model ?? [];
  const activeModel = isModelAvailable(summary, selectedModel) ? selectedModel : ALL_MODELS;

  useEffect(() => {
    if (activeModel === selectedModel) return;
    setSelectedModel(activeModel);
    persistSelectedModel(source, activeModel);
  }, [activeModel, selectedModel, source]);

  const handleModelChange = (model: string) => {
    setSelectedModel(model);
    persistSelectedModel(source, model);
  };

  return (
    <div className="grid h-full grid-cols-2 grid-rows-2 gap-x-1 gap-y-2.5">
      <SummaryCard
        label="今日"
        usage={periodUsage(summary?.today, summary?.today_by_model, activeModel)}
        models={models}
        selectedModel={activeModel}
        onModelChange={handleModelChange}
      />
      <SummaryCard
        label="本周"
        usage={periodUsage(summary?.this_week, summary?.this_week_by_model, activeModel)}
        models={models}
        selectedModel={activeModel}
        onModelChange={handleModelChange}
      />
      <SummaryCard
        label="本月"
        usage={periodUsage(summary?.this_month, summary?.this_month_by_model, activeModel)}
        models={models}
        selectedModel={activeModel}
        onModelChange={handleModelChange}
      />
      <SummaryCard
        label="总计"
        usage={periodUsage(summary?.total, summary?.by_model, activeModel)}
        models={models}
        selectedModel={activeModel}
        onModelChange={handleModelChange}
      />
    </div>
  );
}
