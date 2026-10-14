import { useState } from "react";
import { abbrevModel, formatTokens } from "../../lib/format";
import { useUnit } from "../../lib/unit";
import {
  computeCost,
  formatCost,
  usePricing,
  type ModelPrice,
} from "../../lib/pricing";
import type { ModelUsage, StatsSummary } from "../../types";

const EMPTY_PRICE: ModelPrice = { input: 0, output: 0, cache: 0, cache_write: 0 };

const PRICE_FIELDS: { key: keyof ModelPrice; label: string }[] = [
  { key: "input", label: "输入" },
  { key: "output", label: "输出" },
  { key: "cache", label: "缓存读" },
  { key: "cache_write", label: "缓存写" },
];

/** 费用估算完整版：每模型三个单价输入框 + 合计。 */
export default function CostCard({ summary }: { summary?: StatsSummary }) {
  const { unit } = useUnit();
  const [pricing, setPricing] = usePricing();
  const [showOthers, setShowOthers] = useState(false);

  const models = summary?.by_model ?? [];
  const kimiModels = models.filter((m) => m.model.toLowerCase().startsWith("kimi"));
  const otherModels = models.filter((m) => !m.model.toLowerCase().startsWith("kimi"));
  const update = (model: string, key: keyof ModelPrice, value: number) => {
    setPricing({
      ...pricing,
      [model]: { ...(pricing[model] ?? EMPTY_PRICE), [key]: value },
    });
  };

  const totalCost = models.reduce(
    (acc, m) => acc + computeCost(m, pricing[m.model] ?? EMPTY_PRICE),
    0
  );

  const renderRow = (m: ModelUsage) => {
    const price = pricing[m.model] ?? EMPTY_PRICE;
    const cost = computeCost(m, price);
    return (
      <div
        key={m.model}
        className="grid grid-cols-[minmax(0,1fr)_repeat(4,3rem)_4rem] items-center gap-2 text-sm"
      >
        <span
          className="truncate font-mono text-zinc-700 dark:text-zinc-300"
          title={`${m.model} · 用量 ${formatTokens(m.input + m.output + m.cache_read + m.cache_creation, unit)}`}
        >
          {abbrevModel(m.model)}
        </span>
        {PRICE_FIELDS.map((f) => (
          // 非受控输入：受控写法 value={Number 归一后的值} 会把输入中的
          // "0."、"1." 立即吞掉导致小数输不进去。单价表的唯一写入方就是
          // 这些输入框，且每行按模型 key 重挂载，defaultValue 总是新鲜的。
          <input
            key={f.key}
            type="number"
            min={0}
            step={0.01}
            defaultValue={price[f.key] || ""}
            placeholder="0"
            onChange={(e) =>
              update(m.model, f.key, Number(e.target.value) || 0)
            }
            className="w-full min-w-0 rounded border border-zinc-300 bg-white px-1.5 py-0.5 text-right font-mono text-zinc-800 outline-none focus:border-zinc-400 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-200 dark:focus:border-zinc-600 [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
          />
        ))}
        <span className="text-right font-mono text-zinc-700 dark:text-zinc-300">
          {formatCost(cost)}
        </span>
      </div>
    );
  };

  return (
    <section className="min-w-0 h-full rounded-xl border border-zinc-200 bg-white px-1.5 py-2.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] dark:border-zinc-800/60 dark:bg-zinc-900/50">
      <div className="mb-1.5 flex items-baseline justify-between gap-3">
        <p className="text-base text-zinc-500 dark:text-zinc-400">费用估算</p>
        <p className="font-mono text-base text-emerald-600 dark:text-emerald-300">{formatCost(totalCost)}</p>
      </div>
      {models.length === 0 ? (
        <p className="text-base text-zinc-400 dark:text-zinc-600">暂无数据</p>
      ) : (
        <div className="space-y-1.5">
          <div className="grid grid-cols-[minmax(0,1fr)_repeat(4,3rem)_4rem] items-center gap-2 text-xs text-zinc-400 dark:text-zinc-600">
            <span>模型</span>
            {PRICE_FIELDS.map((f) => (
              <span key={f.key} className="text-right">
                {f.label}
              </span>
            ))}
            <span className="text-right">费用</span>
          </div>
          {kimiModels.map(renderRow)}
          {otherModels.length > 0 && kimiModels.length > 0 && (
            <button
              type="button"
              onClick={() => setShowOthers((v) => !v)}
              className="text-left text-sm text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200"
            >
              {showOthers
                ? "▾ 收起其他模型"
                : `▸ 展开其他供应商模型（${otherModels.length}）`}
            </button>
          )}
          {otherModels.length > 0 &&
            (kimiModels.length === 0 || showOthers) &&
            otherModels.map(renderRow)}
          <p className="pt-0.5 text-xs leading-4 text-zinc-400 dark:text-zinc-600">
            单价为人民币 元/百万 tokens，自行按供应商价格页填写，本地保存。
            订阅制模型（如 kimi-code 托管套餐）不按量计费，留 0 即可。
          </p>
        </div>
      )}
    </section>
  );
}
