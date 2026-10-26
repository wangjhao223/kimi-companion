import { useQuery } from "@tanstack/react-query";
import {
  Bar,
  CartesianGrid,
  ComposedChart,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { STATS_APIS } from "../../lib/api";
import { formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import { useTheme } from "../../lib/theme";
import type { AgentSource, RatePoint } from "../../types";

/** 坐标轴/网格/悬浮框的界面色随主题切换（与 TrendChart 同一套）；数据线颜色两主题通用。 */
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

function formatMinute(ms: number): string {
  const d = new Date(ms);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}

/** 最近一个有流量的桶（RPM 或 TPM 非零），全零返回 null。 */
function latestActive(points: RatePoint[]): RatePoint | null {
  for (let i = points.length - 1; i >= 0; i--) {
    const p = points[i];
    if (p.requests > 0 || p.tokens > 0) return p;
  }
  return null;
}

export default function RateChart({ source }: { source: AgentSource }) {
  const { unit } = useUnit();
  const { resolved } = useTheme();
  const ui = CHART_UI_COLORS[resolved];
  const ratesQuery = useQuery({
    queryKey: ["stats-rates", source, 60],
    queryFn: () => STATS_APIS[source].rates(60),
    refetchInterval: 10_000,
  });
  const data = ratesQuery.data ?? [];
  const latest = latestActive(data);
  const isEmpty = !ratesQuery.isLoading && latest === null;

  return (
    <section className="min-w-0 h-full rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <div className="mb-1.5 flex flex-wrap items-center justify-between gap-x-2 gap-y-1">
        <p className="text-sm text-zinc-500 dark:text-zinc-400">近 60 分钟速率</p>
        <div className="flex min-w-0 flex-wrap items-center justify-end gap-x-4 gap-y-1 text-sm text-zinc-500">
          {latest ? (
            <span className="text-base text-zinc-600 dark:text-zinc-300">
              当前 TPM {formatTokens(latest.tokens, unit)} · RPM {latest.requests}
              <span className="ml-1 text-sm text-zinc-400 dark:text-zinc-500">
                ({formatMinute(latest.minute_start_ms)})
              </span>
            </span>
          ) : (
            <span className="text-zinc-400 dark:text-zinc-500">当前暂无流量</span>
          )}
          <span className="flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-sky-400" /> TPM
          </span>
          <span className="flex items-center gap-1.5">
            <span className="h-2 w-2 rounded-full bg-amber-400" /> RPM
          </span>
        </div>
      </div>
      {ratesQuery.isLoading ? (
        <p className="py-8 text-center text-base text-zinc-400 dark:text-zinc-600">加载中…</p>
      ) : isEmpty ? (
        <p className="py-8 text-center text-base text-zinc-400 dark:text-zinc-600">
          近 60 分钟暂无用量记录
        </p>
      ) : (
        <div className="h-32">
          <ResponsiveContainer width="100%" height="100%">
            <ComposedChart data={data} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
              <CartesianGrid stroke={ui.grid} strokeDasharray="3 3" vertical={false} />
              <XAxis
                dataKey="minute_start_ms"
                tick={{ fill: ui.tick, fontSize: 13 }}
                tickFormatter={(ms: number) => formatMinute(ms)}
                tickLine={false}
                axisLine={{ stroke: ui.axis }}
                minTickGap={48}
              />
              <YAxis
                yAxisId="tpm"
                tick={{ fill: ui.tick, fontSize: 13 }}
                tickFormatter={(v: number) => formatTokens(v, unit)}
                tickLine={false}
                axisLine={false}
                width={44}
              />
              <YAxis
                yAxisId="rpm"
                orientation="right"
                tick={{ fill: ui.tick, fontSize: 13 }}
                tickLine={false}
                axisLine={false}
                allowDecimals={false}
                width={32}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: ui.tooltipBg,
                  border: `1px solid ${ui.tooltipBorder}`,
                  borderRadius: 8,
                  fontSize: 13,
                }}
                labelStyle={{ color: ui.tooltipLabel }}
                labelFormatter={(ms) => formatMinute(Number(ms))}
                formatter={(value, name) =>
                  name === "tokens"
                    ? [formatTokens(Number(value), unit), "TPM"]
                    : [Number(value), "RPM"]
                }
              />
              <Bar yAxisId="tpm" dataKey="tokens" fill="#38bdf8" radius={[2, 2, 0, 0]} />
              <Line
                yAxisId="rpm"
                type="monotone"
                dataKey="requests"
                stroke="#fbbf24"
                dot={false}
                strokeWidth={1.5}
              />
            </ComposedChart>
          </ResponsiveContainer>
        </div>
      )}
    </section>
  );
}
