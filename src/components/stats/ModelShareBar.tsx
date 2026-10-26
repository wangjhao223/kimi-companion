import { useState } from "react";
import { abbrevModel, formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import type { StatsSummary } from "../../types";

const LEGEND_LIMIT = 4;

const MODEL_COLORS = [
  "bg-emerald-500",
  "bg-teal-500",
  "bg-cyan-500",
  "bg-sky-500",
  "bg-violet-500",
  "bg-amber-500",
];

export default function ModelShareBar({ summary }: { summary?: StatsSummary }) {
  const { unit } = useUnit();
  const [expanded, setExpanded] = useState(false);
  const models = summary?.by_model ?? [];
  const hiddenCount = Math.max(0, models.length - LEGEND_LIMIT);
  const legend = (expanded ? models : models.slice(0, LEGEND_LIMIT)).map(
    (model, index) => ({ model, index })
  );
  const grand = models.reduce(
    (acc, m) => acc + m.input + m.output + m.cache_read + m.cache_creation,
    0
  );
  return (
    <section className="min-w-0 h-full rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="mb-1.5 text-sm text-zinc-500 dark:text-zinc-400">按模型占比</p>
      {models.length === 0 || grand === 0 ? (
        <p className="text-base text-zinc-400 dark:text-zinc-600">暂无数据</p>
      ) : (
        <>
          <div className="flex h-3 w-full min-w-0 overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
            {models.map((m, i) => {
              const total = m.input + m.output + m.cache_read + m.cache_creation;
              const pct = (total / grand) * 100;
              return (
                <div
                  key={m.model}
                  className={`h-full shrink-0 ${MODEL_COLORS[i % MODEL_COLORS.length]}`}
                  style={{ width: `${pct}%` }}
                  title={`${m.model}: ${formatTokens(total, unit)}（${pct.toFixed(1)}%）`}
                />
              );
            })}
          </div>
          <div className="mt-2 flex min-w-0 flex-wrap gap-x-3 gap-y-1">
            {legend.map(({ model: m, index: i }) => {
              const total = m.input + m.output + m.cache_read + m.cache_creation;
              const pct = (total / grand) * 100;
              return (
                <div
                  key={m.model}
                  className="flex min-w-0 max-w-full items-center gap-1.5 text-base leading-tight"
                  title={`${m.model}: ${formatTokens(total, unit)}（${pct.toFixed(1)}%）`}
                >
                  <span
                    className={`h-2.5 w-2.5 shrink-0 rounded-sm ${MODEL_COLORS[i % MODEL_COLORS.length]}`}
                  />
                  <span className="min-w-0 truncate font-mono text-zinc-700 dark:text-zinc-300">
                    {abbrevModel(m.model)}
                  </span>
                  <span className="shrink-0 whitespace-nowrap text-zinc-500 dark:text-zinc-400">
                    {formatTokens(total, unit)}（{pct.toFixed(1)}%）
                  </span>
                </div>
              );
            })}
          </div>
          {hiddenCount > 0 && (
            <button
              type="button"
              aria-expanded={expanded}
              onClick={() => setExpanded((value) => !value)}
              className="mt-1.5 text-base text-zinc-500 transition-colors hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200"
            >
              {expanded ? "收起次要模型" : `展开其余模型（${hiddenCount}）`}
            </button>
          )}
        </>
      )}
    </section>
  );
}
