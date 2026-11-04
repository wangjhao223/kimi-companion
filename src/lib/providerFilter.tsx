import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import { useQuery } from "@tanstack/react-query";
import { getAvailableModels } from "./api";
import type { AgentSource, StatsSummary } from "../types";

export const OTHER_PROVIDER = "__other__";

export function providerLabel(provider: string): string {
  if (provider === OTHER_PROVIDER) return "其他";
  if (provider === "kimi-code") return "Kimi Code";
  if (provider === "openai") return "OpenAI";
  return provider;
}

function rawProvider(model: string): string {
  const slash = model.lastIndexOf("/");
  return slash > 0 ? model.slice(0, slash) : model.split("-")[0].toLowerCase();
}

function unique(values: string[]): string[] {
  return [...new Set(values)];
}

/** 当前供应商目录：Kimi 以 provider 配置为准，Codex 统一归 OpenAI。
 *  已删除供应商的历史模型全部归“其他”，旧供应商名本身不出现在选项里。 */
export function useProviderCatalog(
  summary: StatsSummary | undefined,
  source: AgentSource,
  distro?: string
) {
  const hasCatalog = source === "kimi" || source === "kimi-win";
  const catalogQuery = useQuery({
    queryKey: ["available-models", source, source === "kimi" ? (distro ?? null) : null],
    queryFn: () => getAvailableModels(source, source === "kimi" ? distro : undefined),
    enabled: source === "kimi-win" || (source === "kimi" && Boolean(distro)),
    refetchInterval: 30_000,
    retry: 1,
  });

  const usageModels = useMemo(
    () => unique((summary?.by_model ?? []).map((m) => m.model)),
    [summary]
  );
  const currentProviders = useMemo(() => {
    if (!hasCatalog) return usageModels.length > 0 ? ["openai"] : [];
    const models = catalogQuery.isSuccess ? (catalogQuery.data ?? []) : usageModels;
    return unique(models.map(rawProvider).filter(Boolean)).sort();
  }, [catalogQuery.data, catalogQuery.isSuccess, hasCatalog, usageModels]);

  const providerOf = useCallback(
    (model: string) => {
      if (!hasCatalog) return "openai";
      const provider = rawProvider(model);
      return currentProviders.includes(provider) ? provider : OTHER_PROVIDER;
    },
    [currentProviders, hasCatalog]
  );

  const providers = useMemo(() => {
    const hasOther = usageModels.some((model) => providerOf(model) === OTHER_PROVIDER);
    return hasOther ? [...currentProviders, OTHER_PROVIDER] : currentProviders;
  }, [currentProviders, providerOf, usageModels]);

  return {
    providers,
    providerOf,
    ready: hasCatalog ? catalogQuery.isSuccess : summary !== undefined,
  };
}

// 按 key 共享的订阅式选择存储：token 卡与费用卡各自同步，重启后恢复。
const selections = new Map<string, string[]>();
const listeners = new Map<string, Set<() => void>>();

function readSelection(key: string): string[] {
  const cached = selections.get(key);
  if (cached) return cached;
  let value: string[] = [];
  try {
    const raw = localStorage.getItem(key);
    if (raw) {
      const parsed: unknown = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        value = unique(parsed.filter((v): v is string => typeof v === "string"));
      }
    }
  } catch {
    // 存储不可用或内容损坏时回退全部。
  }
  selections.set(key, value);
  return value;
}

function subscribe(key: string, listener: () => void): () => void {
  const group = listeners.get(key) ?? new Set<() => void>();
  group.add(listener);
  listeners.set(key, group);
  return () => {
    group.delete(listener);
    if (group.size === 0) listeners.delete(key);
  };
}

function writeSelection(key: string, value: string[]) {
  const next = unique(value);
  selections.set(key, next);
  try {
    localStorage.setItem(key, JSON.stringify(next));
  } catch {
    // 写入失败仍保留当前会话内的选择。
  }
  listeners.get(key)?.forEach((listener) => listener());
}

/** scope 为 summary 或 cost，四种数据源分别持久化。 */
export function useProviderSelection(
  scope: "summary" | "cost",
  source: AgentSource,
  providers: string[],
  ready: boolean
) {
  const key = `kimi-companion.${scope}-providers.${source}`;
  const selected = useSyncExternalStore(
    useCallback((listener) => subscribe(key, listener), [key]),
    useCallback(() => readSelection(key), [key])
  );
  const setSelected = useCallback((next: string[]) => writeSelection(key, next), [key]);

  useEffect(() => {
    if (!ready || selected.length === 0) return;
    const available = new Set(providers);
    const next = selected.filter((provider) => available.has(provider));
    if (next.length !== selected.length) writeSelection(key, next);
  }, [key, providers, ready, selected]);

  return [selected, setSelected] as const;
}

/** 供应商复选下拉；空数组表示全部。 */
export function ProviderFilter({
  label,
  providers,
  selected,
  onChange,
}: {
  label: string;
  providers: string[];
  selected: string[];
  onChange: (providers: string[]) => void;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const all = selected.length === 0;

  useEffect(() => {
    if (!open) return;
    const pointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const keyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setOpen(false);
      triggerRef.current?.focus();
    };
    document.addEventListener("pointerdown", pointerDown);
    document.addEventListener("keydown", keyDown);
    return () => {
      document.removeEventListener("pointerdown", pointerDown);
      document.removeEventListener("keydown", keyDown);
    };
  }, [open]);

  const summary = all
    ? "全部"
    : selected.length === 1
      ? providerLabel(selected[0])
      : `已选 ${selected.length}`;
  const title = all ? "全部供应商" : selected.map(providerLabel).join("、");

  return (
    <div ref={rootRef} className="relative shrink-0">
      <button
        ref={triggerRef}
        type="button"
        aria-label={`${label}供应商筛选`}
        aria-haspopup="true"
        aria-expanded={open}
        title={title}
        onClick={() => setOpen((value) => !value)}
        className="flex max-w-[5rem] min-w-0 items-center gap-0.5 rounded-md border border-zinc-300 bg-white px-1 py-0.5 text-xs text-zinc-600 outline-none transition-colors hover:border-zinc-400 focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:border-zinc-600 dark:focus:border-zinc-500"
      >
        <span className="min-w-0 truncate">{summary}</span>
        <span aria-hidden="true" className="shrink-0">▾</span>
      </button>
      {open && (
        <div
          role="group"
          aria-label={`${label}供应商筛选`}
          className="absolute right-0 top-full z-50 mt-1 max-h-56 w-40 max-w-[calc(100vw-1rem)] overflow-y-auto rounded-md border border-zinc-200 bg-white py-1 text-xs shadow-lg dark:border-zinc-700 dark:bg-zinc-800"
        >
          <label className="flex cursor-pointer items-center gap-1.5 px-2 py-1 text-zinc-700 hover:bg-zinc-100 dark:text-zinc-200 dark:hover:bg-zinc-700/60">
            <input
              type="checkbox"
              checked={all}
              onChange={() => onChange([])}
              className="h-3 w-3 shrink-0 accent-emerald-600 dark:accent-emerald-400"
            />
            <span>全部供应商</span>
          </label>
          {providers.map((provider) => (
            <label
              key={provider}
              title={providerLabel(provider)}
              className="flex cursor-pointer items-center gap-1.5 px-2 py-1 text-zinc-600 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-700/60"
            >
              <input
                type="checkbox"
                checked={selected.includes(provider)}
                onChange={() =>
                  onChange(
                    selected.includes(provider)
                      ? selected.filter((item) => item !== provider)
                      : [...selected, provider]
                  )
                }
                className="h-3 w-3 shrink-0 accent-emerald-600 dark:accent-emerald-400"
              />
              <span className="truncate">{providerLabel(provider)}</span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
}
