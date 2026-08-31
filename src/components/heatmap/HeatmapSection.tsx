import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getCodexHeatmap, getStatsHeatmap } from "../../lib/api";
import { formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import type { AgentSource, HeatmapDay } from "../../types";

type Dimension = "total" | "output" | "cache";

const DIMENSION_LABELS: Record<Dimension, string> = {
  total: "总量",
  output: "输出",
  cache: "缓存",
};

/** 5 档色阶：0 空档 zinc，1~4 emerald 递增。 */
const LEVEL_COLORS = [
  "bg-zinc-800",
  "bg-emerald-900",
  "bg-emerald-700",
  "bg-emerald-500",
  "bg-emerald-300",
];

const DAYS = 365;
const DAY_MS = 86_400_000;
const MONTH_LABELS = [
  "1月", "2月", "3月", "4月", "5月", "6月",
  "7月", "8月", "9月", "10月", "11月", "12月",
];

function fmtDate(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

function dayValue(day: HeatmapDay | undefined, dim: Dimension): number {
  if (!day) return 0;
  if (dim === "total") return day.total;
  if (dim === "output") return day.output;
  return day.cache_read + day.cache_creation;
}

interface Cell {
  /** null 表示网格范围之外的占位（第一周周日之前的格子） */
  date: string | null;
  day?: HeatmapDay;
}

/** 年度用量热力图区块（占网格一整行，3 个单元宽）。source 决定数据源（kimi / codex）。 */
export default function HeatmapSection({ source }: { source: AgentSource }) {
  const [dim, setDim] = useState<Dimension>("total");
  const { unit } = useUnit();

  const heatmapQuery = useQuery({
    queryKey: ["stats-heatmap", source, DAYS],
    queryFn: () =>
      source === "codex" ? getCodexHeatmap(DAYS) : getStatsHeatmap(DAYS),
    refetchInterval: 60_000,
  });

  // 周 × 星期网格：周日为第一行，列尾对齐到今天，列首回溯到周日
  const { weeks, maxValue } = useMemo(() => {
    const byDate = new Map<string, HeatmapDay>();
    for (const d of heatmapQuery.data ?? []) byDate.set(d.date, d);

    const end = new Date();
    const start = new Date(end.getTime() - (DAYS - 1) * DAY_MS);
    start.setDate(start.getDate() - start.getDay()); // 回溯到周日（getDay: 0=周日）

    const weeks: Cell[][] = [];
    let maxValue = 0;
    const cursor = new Date(start);
    while (cursor <= end) {
      const week: Cell[] = [];
      for (let row = 0; row < 7; row++) {
        if (cursor > end) break;
        const date = fmtDate(cursor);
        const day = byDate.get(date);
        const v = dayValue(day, dim);
        if (v > maxValue) maxValue = v;
        week.push({ date, day });
        cursor.setDate(cursor.getDate() + 1);
      }
      weeks.push(week);
    }
    return { weeks, maxValue };
  }, [heatmapQuery.data, dim]);

  const levelOf = (v: number): number =>
    v <= 0 || maxValue <= 0 ? 0 : Math.min(4, Math.ceil((v / maxValue) * 4));

  return (
    <>
      {heatmapQuery.error && (
        <p className="mb-2.5 rounded-md border border-red-900/60 bg-red-950/40 px-3 py-2 text-sm text-red-300">
          {(heatmapQuery.error as Error).message}
        </p>
      )}

      <section className="rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
        <div className="mb-2 flex flex-wrap items-center justify-between gap-2.5">
          <p className="text-sm text-zinc-400">
            近一年每天的 token 用量（本地时区）
          </p>
          <div className="flex rounded-lg border border-zinc-800 p-0.5">
            {(Object.keys(DIMENSION_LABELS) as Dimension[]).map((d) => (
              <button
                key={d}
                className={`rounded-md px-3 py-1 text-sm ${
                  dim === d
                    ? "bg-emerald-600 text-white"
                    : "text-zinc-400 hover:text-zinc-200"
                }`}
                onClick={() => setDim(d)}
              >
                {DIMENSION_LABELS[d]}
              </button>
            ))}
          </div>
        </div>
        {heatmapQuery.isLoading ? (
          <p className="py-16 text-center text-sm text-zinc-600">加载中…</p>
        ) : (
          <div className="overflow-x-auto pb-1">
            <div className="inline-flex gap-[3px]">
              {/* 左侧星期标签（一/三/五） */}
              <div className="mr-1 flex flex-col gap-[3px] text-[10px] leading-3 text-zinc-600">
                {["日", "一", "二", "三", "四", "五", "六"].map((label, i) => (
                  <div key={label} className="flex h-3 items-center">
                    {i % 2 === 1 ? label : ""}
                  </div>
                ))}
              </div>
              {weeks.map((week, wi) => {
                const first = week.find((c) => c.date);
                const prevFirst =
                  wi > 0 ? weeks[wi - 1].find((c) => c.date) : undefined;
                const monthLabel =
                  first?.date &&
                  first.date.slice(0, 7) !== prevFirst?.date?.slice(0, 7)
                    ? MONTH_LABELS[Number(first.date.slice(5, 7)) - 1]
                    : "";
                return (
                  <div key={wi} className="flex flex-col">
                    <div className="mb-1 h-4 w-0 overflow-visible whitespace-nowrap text-[10px] text-zinc-500">
                      {monthLabel}
                    </div>
                    <div className="flex flex-col gap-[3px]">
                      {week.map((cell, ci) =>
                        cell.date === null ? (
                          <div key={ci} className="h-3 w-3" />
                        ) : (
                          <div
                            key={cell.date}
                            className={`h-3 w-3 rounded-[2px] ${LEVEL_COLORS[levelOf(dayValue(cell.day, dim))]}`}
                            title={
                              cell.day
                                ? `${cell.date}\n输入 ${formatTokens(cell.day.input, unit)} · 输出 ${formatTokens(cell.day.output, unit)} · 缓存 ${formatTokens(cell.day.cache_read + cell.day.cache_creation, unit)}\n总计 ${formatTokens(cell.day.total, unit)}`
                                : `${cell.date}\n无记录`
                            }
                          />
                        )
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        <div className="mt-2 flex items-center justify-end gap-1.5 text-xs text-zinc-500">
          <span>少</span>
          {LEVEL_COLORS.map((c) => (
            <span key={c} className={`h-3 w-3 rounded-[2px] ${c}`} />
          ))}
          <span>多</span>
          {maxValue > 0 && (
            <span className="ml-3 text-zinc-600">
              最高单日 {formatTokens(maxValue, unit)}（{DIMENSION_LABELS[dim]}）
            </span>
          )}
        </div>
      </section>
    </>
  );
}
