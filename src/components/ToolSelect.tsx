import { useState } from "react";

/** 平台页内的程序选择：Kimi Code / Codex。 */
export type Tool = "kimi-code" | "codex";

const TOOLS: { key: Tool; label: string }[] = [
  { key: "kimi-code", label: "Kimi Code" },
  { key: "codex", label: "Codex" },
];

/** 平台页的程序选择状态，按 storageKey 持久化到 localStorage。 */
export function useToolSelection(storageKey: string): [Tool, (t: Tool) => void] {
  const [tool, setTool] = useState<Tool>(() => {
    try {
      const v = localStorage.getItem(storageKey);
      return v === "codex" || v === "kimi-code" ? v : "kimi-code";
    } catch {
      return "kimi-code";
    }
  });
  const select = (t: Tool) => {
    setTool(t);
    try {
      localStorage.setItem(storageKey, t);
    } catch {
      // localStorage 不可用时只改内存态
    }
  };
  return [tool, select];
}

/** 程序下拉选择器；字号兼作小节标题（text-base font-semibold）。 */
export default function ToolSelect({
  value,
  onChange,
}: {
  value: Tool;
  onChange: (t: Tool) => void;
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value as Tool)}
      className="rounded-md border border-zinc-300 bg-white px-2 py-1 text-base font-semibold text-zinc-800 outline-none transition-colors focus:border-zinc-500 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200"
    >
      {TOOLS.map((t) => (
        <option key={t.key} value={t.key}>
          {t.label}
        </option>
      ))}
    </select>
  );
}
