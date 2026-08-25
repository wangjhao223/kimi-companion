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
import { getStatsTrend } from "../../lib/api";
import { formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";

export default function TrendChart() {
  const { unit } = useUnit();
  const trendQuery = useQuery({
    queryKey: ["stats-trend", 30],
    queryFn: () => getStatsTrend(30),
    refetchInterval: 30_000,
  });
  const data = trendQuery.data ?? [];

  return (
    <section className="h-full rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-sm text-zinc-400">近 30 天趋势</p>
        <div className="flex gap-4 text-xs text-zinc-500">
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
        <p className="py-12 text-center text-sm text-zinc-600">加载中…</p>
      ) : (
        <div className="h-40">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
              <CartesianGrid stroke="#27272a" strokeDasharray="3 3" vertical={false} />
              <XAxis
                dataKey="date"
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickFormatter={(d: string) => d.slice(5)}
                tickLine={false}
                axisLine={{ stroke: "#3f3f46" }}
                minTickGap={24}
              />
              <YAxis
                tick={{ fill: "#71717a", fontSize: 11 }}
                tickFormatter={(v: number) => formatTokens(v, unit)}
                tickLine={false}
                axisLine={false}
                width={44}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "#18181b",
                  border: "1px solid #3f3f46",
                  borderRadius: 8,
                  fontSize: 12,
                }}
                labelStyle={{ color: "#a1a1aa" }}
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
