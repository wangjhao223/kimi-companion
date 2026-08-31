import { formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import type { StatsSummary } from "../../types";

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
  const models = summary?.by_model ?? [];
  const grand = models.reduce(
    (acc, m) => acc + m.input + m.output + m.cache_read + m.cache_creation,
    0
  );
  return (
    <section className="h-full rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <p className="mb-2 text-sm text-zinc-400">按模型占比（全部时间）</p>
      {models.length === 0 || grand === 0 ? (
        <p className="text-sm text-zinc-600">暂无数据</p>
      ) : (
        <>
          <div className="flex h-3 w-full overflow-hidden rounded-full bg-zinc-800">
            {models.map((m, i) => {
              const total =
                m.input + m.output + m.cache_read + m.cache_creation;
              return (
                <div
                  key={m.model}
                  className={MODEL_COLORS[i % MODEL_COLORS.length]}
                  style={{ width: `${(total / grand) * 100}%` }}
                  title={`${m.model}: ${formatTokens(total, unit)}`}
                />
              );
            })}
          </div>
          <div className="mt-2 flex flex-wrap gap-x-5 gap-y-1">
            {models.map((m, i) => {
              const total =
                m.input + m.output + m.cache_read + m.cache_creation;
              return (
                <div key={m.model} className="flex items-center gap-2 text-xs">
                  <span
                    className={`h-2.5 w-2.5 rounded-sm ${MODEL_COLORS[i % MODEL_COLORS.length]}`}
                  />
                  <span className="font-mono text-zinc-300">{m.model}</span>
                  <span className="text-zinc-500">
                    {formatTokens(total, unit)}（{((total / grand) * 100).toFixed(1)}%）
                  </span>
                </div>
              );
            })}
          </div>
        </>
      )}
    </section>
  );
}
