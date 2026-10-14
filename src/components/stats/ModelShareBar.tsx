import { Cell, Pie, PieChart, ResponsiveContainer, Tooltip } from "recharts";
import { formatTokens } from "../../lib/format";
import { useTheme } from "../../lib/theme";
import { useUnit } from "../../lib/unit";
import type { StatsSummary } from "../../types";

const MODEL_COLORS = ["#10b981", "#14b8a6", "#06b6d4", "#0ea5e9", "#8b5cf6", "#f59e0b"];

const TOOLTIP_COLORS = {
  light: { bg: "#ffffff", border: "#e4e4e7", label: "#52525b" },
  dark: { bg: "#18181b", border: "#3f3f46", label: "#a1a1aa" },
} as const;

export default function ModelShareBar({ summary }: { summary?: StatsSummary }) {
  const { unit } = useUnit();
  const { resolved } = useTheme();
  const tooltip = TOOLTIP_COLORS[resolved];
  const models = summary?.by_model ?? [];
  const grand = models.reduce(
    (acc, m) => acc + m.input + m.output + m.cache_read + m.cache_creation,
    0
  );
  return (
    <section className="min-w-0 h-full rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="mb-1.5 text-base text-zinc-500 dark:text-zinc-400">按模型占比</p>
      {models.length === 0 || grand === 0 ? (
        <p className="text-base text-zinc-400 dark:text-zinc-600">暂无数据</p>
      ) : (
        <>
          <div className="h-32">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={models}
                  dataKey={(m) => m.input + m.output + m.cache_read + m.cache_creation}
                  nameKey="model"
                  innerRadius="55%"
                  outerRadius="90%"
                  strokeWidth={0}
                  isAnimationActive={false}
                >
                  {models.map((m, i) => (
                    <Cell key={m.model} fill={MODEL_COLORS[i % MODEL_COLORS.length]} />
                  ))}
                </Pie>
                <Tooltip
                  contentStyle={{
                    backgroundColor: tooltip.bg,
                    border: `1px solid ${tooltip.border}`,
                    borderRadius: 8,
                    fontSize: 12,
                  }}
                  labelStyle={{ color: tooltip.label }}
                  formatter={(value, _name, item) => [
                    `${formatTokens(Number(value), unit)}（${((Number(value) / grand) * 100).toFixed(1)}%）`,
                    item.payload.model,
                  ]}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
        </>
      )}
    </section>
  );
}
