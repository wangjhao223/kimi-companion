import { useQuery } from "@tanstack/react-query";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { STATS_APIS } from "../../lib/api";
import { formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import { useTheme } from "../../lib/theme";
import type { AgentSource } from "../../types";

/** 坐标轴/网格/悬浮框的界面色随主题切换（亮色一套、暗色一套）；数据线颜色两主题通用。 */
const CHART_UI_COLORS = {
  light: {
    tick: "#71717a", // zinc-500
    grid: "#e4e4e7", // zinc-200
    axis: "#d4d4d8", // zinc-300
    tooltipBg: "#ffffff",
    tooltipBorder: "#e4e4e7", // zinc-200
    tooltipLabel: "#52525b", // zinc-600
  },
  dark: {
    tick: "#71717a", // zinc-500
    grid: "#27272a", // zinc-800
    axis: "#3f3f46", // zinc-700
    tooltipBg: "#18181b", // zinc-900
    tooltipBorder: "#3f3f46", // zinc-700
    tooltipLabel: "#a1a1aa", // zinc-400
  },
} as const;

export default function TrendChart({ source }: { source: AgentSource }) {
  const { unit } = useUnit();
  const { resolved } = useTheme();
  const ui = CHART_UI_COLORS[resolved];
  const trendQuery = useQuery({
    queryKey: ["stats-trend", source, 30],
    queryFn: () => STATS_APIS[source].trend(30),
    refetchInterval: 30_000,
  });
  const data = trendQuery.data ?? [];

  return (
    <section className="h-full rounded-xl border border-zinc-200 bg-white p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-base text-zinc-500 dark:text-zinc-400">近 30 天趋势</p>
        <div className="flex gap-4 text-sm text-zinc-500">
          <span className="flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-emerald-400" /> 输入
          </span>
          <span className="flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-sky-400" /> 输出
          </span>
          <span className="flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-violet-400" /> 缓存
          </span>
        </div>
      </div>
      {trendQuery.isLoading ? (
        <p className="py-12 text-center text-base text-zinc-400 dark:text-zinc-600">加载中…</p>
      ) : (
        <div className="h-40">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
              <CartesianGrid stroke={ui.grid} strokeDasharray="3 3" vertical={false} />
              <XAxis
                dataKey="date"
                tick={{ fill: ui.tick, fontSize: 12 }}
                tickFormatter={(d: string) => d.slice(5)}
                tickLine={false}
                axisLine={{ stroke: ui.axis }}
                minTickGap={24}
              />
              <YAxis
                tick={{ fill: ui.tick, fontSize: 12 }}
                tickFormatter={(v: number) => formatTokens(v, unit)}
                tickLine={false}
                axisLine={false}
                width={44}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: ui.tooltipBg,
                  border: `1px solid ${ui.tooltipBorder}`,
                  borderRadius: 8,
                  fontSize: 12,
                }}
                labelStyle={{ color: ui.tooltipLabel }}
                formatter={(value, name) => [
                  formatTokens(Number(value), unit),
                  name === "input" ? "输入" : name === "output" ? "输出" : "缓存",
                ]}
              />
              <Line type="monotone" dataKey="input" stroke="#34d399" dot={false} strokeWidth={1.5} />
              <Line type="monotone" dataKey="output" stroke="#38bdf8" dot={false} strokeWidth={1.5} />
              <Line type="monotone" dataKey="cache" stroke="#a78bfa" dot={false} strokeWidth={1.5} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </section>
  );
}
