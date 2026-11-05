import { useState } from "react";
import type { ModelPrice, PricingRule } from "../../lib/pricing";

type PriceField = "input" | "output" | "cache" | "cache_write";

const PRICE_FIELDS: { key: PriceField; label: string }[] = [
  { key: "input", label: "输入" },
  { key: "output", label: "输出" },
  { key: "cache", label: "缓存读" },
  { key: "cache_write", label: "缓存写" },
];

function newRule(price: ModelPrice): PricingRule {
  return {
    id: crypto.randomUUID(),
    input: price.input,
    output: price.output,
    cache: price.cache,
    cache_write: price.cache_write ?? 0,
  };
}

/** 单模型条件计费：展开后可启用每日时段/上下文区间规则。 */
export default function ConditionalPricing({
  price,
  onChange,
}: {
  price: ModelPrice;
  onChange: (price: ModelPrice) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const enabled = price.advanced_enabled ?? false;
  const rules = price.rules ?? [];

  const updateRule = (id: string, patch: Partial<PricingRule>) => {
    onChange({
      ...price,
      rules: rules.map((rule) => (rule.id === id ? { ...rule, ...patch } : rule)),
    });
  };

  return (
    <div className="shrink-0">
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
        className="whitespace-nowrap text-left text-xs text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200"
      >
        {expanded ? "▾" : "▸"} 条件计费{enabled ? "（已启用）" : ""}
      </button>
      {expanded && (
        <div className="absolute left-0 top-full z-50 mt-1 w-[32rem] max-w-[calc(100vw-2rem)] rounded-md border border-zinc-200 bg-zinc-50 p-2 shadow-lg dark:border-zinc-800 dark:bg-zinc-950/95">
          <label className="flex items-center gap-1.5 text-sm text-zinc-600 dark:text-zinc-300">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) =>
                onChange({ ...price, advanced_enabled: e.target.checked })
              }
              className="accent-emerald-600 dark:accent-emerald-400"
            />
            为此模型启用条件计费
          </label>
          {enabled && (
            <div className="mt-2 space-y-2">
              {rules.map((rule, index) => (
                <div
                  key={rule.id}
                  className="min-w-0 rounded border border-zinc-200 bg-white p-1.5 dark:border-zinc-800 dark:bg-zinc-900"
                >
                  <div className="mb-1 flex items-center justify-between text-xs text-zinc-500">
                    <span>规则 {index + 1}</span>
                    <button
                      type="button"
                      onClick={() =>
                        onChange({
                          ...price,
                          rules: rules.filter((item) => item.id !== rule.id),
                        })
                      }
                      className="text-red-500 hover:text-red-600"
                    >
                      删除
                    </button>
                  </div>
                  <div className="grid grid-cols-2 gap-1 text-xs text-zinc-500 md:grid-cols-4">
                    <label>
                      开始
                      <input
                        type="time"
                        value={rule.start_time ?? ""}
                        onChange={(e) =>
                          updateRule(rule.id, { start_time: e.target.value || undefined })
                        }
                        className="mt-0.5 w-full min-w-0 rounded border border-zinc-300 bg-white px-1 py-0.5 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200"
                      />
                    </label>
                    <label>
                      结束
                      <input
                        type="time"
                        value={rule.end_time ?? ""}
                        onChange={(e) =>
                          updateRule(rule.id, { end_time: e.target.value || undefined })
                        }
                        className="mt-0.5 w-full min-w-0 rounded border border-zinc-300 bg-white px-1 py-0.5 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200"
                      />
                    </label>
                    <label>
                      最小上下文
                      <input
                        type="number"
                        min={0}
                        step={1000}
                        defaultValue={rule.min_context ?? ""}
                        placeholder="不限"
                        onChange={(e) =>
                          updateRule(rule.id, {
                            min_context: e.target.value ? Number(e.target.value) : undefined,
                          })
                        }
                        className="mt-0.5 w-full min-w-0 rounded border border-zinc-300 bg-white px-1 py-0.5 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200"
                      />
                    </label>
                    <label>
                      最大上下文
                      <input
                        type="number"
                        min={0}
                        step={1000}
                        defaultValue={rule.max_context ?? ""}
                        placeholder="不限"
                        onChange={(e) =>
                          updateRule(rule.id, {
                            max_context: e.target.value ? Number(e.target.value) : undefined,
                          })
                        }
                        className="mt-0.5 w-full min-w-0 rounded border border-zinc-300 bg-white px-1 py-0.5 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200"
                      />
                    </label>
                  </div>
                  <div className="mt-1 grid grid-cols-2 gap-1 text-xs text-zinc-500 md:grid-cols-4">
                    {PRICE_FIELDS.map((field) => (
                      <label key={field.key}>
                        {field.label}单价
                        <input
                          type="number"
                          min={0}
                          step={0.01}
                          defaultValue={rule[field.key] ?? ""}
                          placeholder="0"
                          onChange={(e) =>
                            updateRule(rule.id, {
                              [field.key]: Number(e.target.value) || 0,
                            })
                          }
                          className="mt-0.5 w-full min-w-0 rounded border border-zinc-300 bg-white px-1 py-0.5 text-right font-mono dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200"
                        />
                      </label>
                    ))}
                  </div>
                </div>
              ))}
              <button
                type="button"
                onClick={() =>
                  onChange({ ...price, rules: [...rules, newRule(price)] })
                }
                className="rounded border border-zinc-300 px-2 py-1 text-xs text-zinc-600 hover:bg-zinc-100 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
              >
                添加规则
              </button>
              <p className="text-xs leading-4 text-zinc-400 dark:text-zinc-500">
                规则自上而下取首条匹配；时间按本地时区，支持跨零点。上下文 = 输入 + 缓存读 + 缓存写，最小值含、最大值不含。
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
