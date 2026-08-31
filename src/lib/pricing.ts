/** 模型单价表：人民币 元 / 百万 tokens，存 localStorage，用户自填。
 *  带订阅机制：savePricing 后所有 usePricing 订阅方同步刷新。 */

import { useSyncExternalStore } from "react";

export interface ModelPrice {
  /** 输入（未命中缓存） */
  input: number;
  /** 输出 */
  output: number;
  /** 缓存读取 */
  cache: number;
  /** 缓存创建（写入）。旧本地数据没有此字段，计价时按 0 处理 */
  cache_write?: number;
}

export type PricingTable = Record<string, ModelPrice>;

const STORAGE_KEY = "kimi-companion.pricing";

export function loadPricing(): PricingTable {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as PricingTable) : {};
  } catch {
    return {};
  }
}

export function savePricing(table: PricingTable): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(table));
  } catch {
    // localStorage 不可用时放弃持久化
  }
  current = table;
  listeners.forEach((l) => l());
}

// ---- 订阅式 store（useSyncExternalStore） ----

/** 内存缓存，保证 getSnapshot 返回稳定引用。 */
let current: PricingTable | null = null;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot(): PricingTable {
  if (current === null) current = loadPricing();
  return current;
}

/** 读取单价表并订阅变更；返回 [表, 保存函数]。 */
export function usePricing(): [PricingTable, (t: PricingTable) => void] {
  const table = useSyncExternalStore(subscribe, getSnapshot);
  return [table, savePricing];
}

export interface TokenUsage {
  input: number;
  output: number;
  cache_read: number;
  cache_creation: number;
}

/** 按单价表估算费用（元）。缓存读与缓存写分开计价。 */
export function computeCost(usage: TokenUsage, price: ModelPrice): number {
  return (
    (usage.input * price.input +
      usage.output * price.output +
      usage.cache_read * price.cache +
      usage.cache_creation * (price.cache_write ?? 0)) /
    1_000_000
  );
}

/** 费用显示：小于 0.01 元时保留 4 位小数，否则 2 位。 */
export function formatCost(v: number): string {
  return "¥" + (v > 0 && v < 0.01 ? v.toFixed(4) : v.toFixed(2));
}

/** 缓存命中率 = 缓存读 / (输入 + 缓存读 + 缓存写)；无输入时返回 null。 */
export function cacheHitRate(u: TokenUsage): number | null {
  const total = u.input + u.cache_read + u.cache_creation;
  if (total <= 0) return null;
  return u.cache_read / total;
}
