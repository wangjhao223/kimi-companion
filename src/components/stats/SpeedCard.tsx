import { useQuery } from "@tanstack/react-query";
import { STATS_APIS } from "../../lib/api";
import type { AgentSource } from "../../types";

/** tok/s 格式化：数值小，一位小数即可，不走 k/M/B 单位。 */
function formatSpeed(v: number): string {
  return v >= 100 ? String(Math.round(v)) : v.toFixed(1);
}

/** 输出速度小卡（1×1）：纯文字显示当前瞬时速度与近 1 小时 / 今日加权均速。
 *  速度数据自支持 duration 的版本安装起向前累计，历史记录没有耗时。 */
export default function SpeedCard({ source }: { source: AgentSource }) {
  const speedQuery = useQuery({
    queryKey: ["stats-speed", source],
    queryFn: () => STATS_APIS[source].speed(),
    refetchInterval: 10_000,
  });
  const stats = speedQuery.data;
  const rows = [
    { label: "近 1 小时", value: stats?.hour_avg_speed },
    { label: "今日", value: stats?.today_avg_speed },
  ];

  return (
    <section className="min-w-0 h-full rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="text-base text-zinc-500">输出速度（当前）</p>
      <p className="mt-0.5 font-mono text-xl font-semibold text-emerald-600 dark:text-emerald-300">
        {stats?.latest_speed != null ? `${formatSpeed(stats.latest_speed)} tok/s` : "—"}
      </p>
      <div className="mt-1.5 space-y-0.5 border-t border-zinc-200 pt-1.5 dark:border-zinc-800/60">
        {rows.map((r) => (
          <div key={r.label} className="flex items-baseline justify-between text-sm">
            <span className="text-zinc-500">{r.label}均速</span>
            <span className="font-mono text-zinc-700 dark:text-zinc-300">
              {r.value != null ? `${formatSpeed(r.value)} tok/s` : "—"}
            </span>
          </div>
        ))}
      </div>
      <p className="mt-1 text-sm text-zinc-500">仅统计新版本之后带耗时的记录</p>
    </section>
  );
}
