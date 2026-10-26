import { useQuery } from "@tanstack/react-query";
import { STATS_APIS } from "../../lib/api";
import { abbrevModel } from "../../lib/format";
import type { AgentSource, ModelSpeedStats } from "../../types";

/** tok/s 格式化：数值小，一位小数即可，不走 k/M/B 单位。 */
function formatSpeed(v: number): string {
  return v >= 100 ? String(Math.round(v)) : v.toFixed(1);
}

function displayName(model: string): string {
  return model.trim() || "未知模型";
}

/** 输出速度小卡（1×1）：每个模型一行显示当前叠加速度（tok/s，同模型并行会话的生成速度
 *  按时间重叠累加）。顺序完全取后端返回；多模型时列表内部滚动。 */
export default function SpeedCard({ source }: { source: AgentSource }) {
  const speedQuery = useQuery({
    queryKey: ["stats-speed", source],
    queryFn: () => STATS_APIS[source].speed(),
    refetchInterval: 10_000,
  });
  const models: ModelSpeedStats[] = speedQuery.data?.models ?? [];

  return (
    <section className="flex h-full min-w-0 flex-col rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <p className="shrink-0 text-sm text-zinc-500">输出速度（当前）</p>
      {models.length === 0 ? (
        <p className="mt-0.5 shrink-0 font-mono text-xl font-semibold text-emerald-600 dark:text-emerald-300">
          —
        </p>
      ) : (
        <div className="mt-1 min-h-0 max-h-32 flex-1 space-y-1 overflow-x-hidden overflow-y-auto pr-0.5">
          {models.map((m, i) => {
            const name = displayName(m.model);
            const text = formatSpeed(m.current_speed);
            return (
              <div
                key={`${m.model}-${i}`}
                className="flex min-w-0 items-baseline justify-between gap-1 border-t border-zinc-200 pt-1 first:border-t-0 first:pt-0 dark:border-zinc-800/60"
              >
                <p
                  className="min-w-0 truncate text-base font-medium leading-tight text-zinc-700 dark:text-zinc-200"
                  title={name}
                >
                  {abbrevModel(name)}
                </p>
                <p
                  className="shrink-0 whitespace-nowrap font-mono text-base leading-tight text-zinc-700 dark:text-zinc-300"
                  title={`${text} tok/s`}
                >
                  {text}
                  <span className="ml-0.5 text-xs text-zinc-500">tok/s</span>
                </p>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
