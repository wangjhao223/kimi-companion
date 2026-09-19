import { useState } from "react";
import WslPage from "./components/wsl/WslPage";
import WindowsPage from "./components/windows/WindowsPage";
import TitleBar from "./components/TitleBar";
import { UnitProvider, useUnit } from "./lib/unit";
import type { Unit } from "./lib/format";
import { ThemeProvider, useTheme } from "./lib/theme";
import type { ThemeMode } from "./lib/theme";

type Tab = "wsl" | "windows";

const TABS: { key: Tab; label: string }[] = [
  { key: "wsl", label: "wsl" },
  { key: "windows", label: "windows" },
];

const UNITS: Unit[] = ["k", "M", "B"];

const THEME_OPTIONS: { key: ThemeMode; label: string }[] = [
  { key: "light", label: "亮" },
  { key: "dark", label: "暗" },
  { key: "system", label: "跟随" },
];

/** 顶部标签选择持久化：重开应用后回到上次所在的标签页。 */
function useTabSelection(): [Tab, (t: Tab) => void] {
  const [tab, setTab] = useState<Tab>(() => {
    try {
      const v = localStorage.getItem("kimi-companion.tab");
      return v === "wsl" || v === "windows" ? v : "wsl";
    } catch {
      return "wsl";
    }
  });
  const select = (t: Tab) => {
    setTab(t);
    try {
      localStorage.setItem("kimi-companion.tab", t);
    } catch {
      // localStorage 不可用时只改内存态
    }
  };
  return [tab, select];
}

function Shell() {
  const [tab, setTab] = useTabSelection();
  const { unit, setUnit } = useUnit();
  const { mode, setMode } = useTheme();

  return (
    <div className="flex min-h-screen flex-col">
      <TitleBar />
      <nav className="sticky top-0 z-10 border-b border-zinc-200 bg-white/80 backdrop-blur dark:border-zinc-800/60 dark:bg-zinc-950/80">
        <div className="mx-auto flex max-w-5xl items-center gap-1 px-5 py-1.5">
          {TABS.map((t) => (
            <button
              key={t.key}
              className={`rounded-md px-3 py-1 text-[13px] font-medium transition-colors ${
                tab === t.key
                  ? "bg-zinc-200 text-emerald-600 dark:bg-zinc-800 dark:text-emerald-300"
                  : "text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-900 dark:hover:text-zinc-200"
              }`}
              onClick={() => setTab(t.key)}
            >
              {t.label}
            </button>
          ))}
          <div className="ml-auto flex items-center gap-2">
            <span className="text-xs text-zinc-400 dark:text-zinc-600">主题</span>
            <div className="flex rounded-md border border-zinc-200 bg-white p-0.5 dark:border-zinc-800 dark:bg-zinc-900/50">
              {THEME_OPTIONS.map((t) => (
                <button
                  key={t.key}
                  className={`rounded px-2 py-0.5 text-xs transition-colors ${
                    mode === t.key
                      ? "bg-emerald-600 text-white"
                      : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200"
                  }`}
                  onClick={() => setMode(t.key)}
                >
                  {t.label}
                </button>
              ))}
            </div>
            <span className="text-xs text-zinc-400 dark:text-zinc-600">单位</span>
            <div className="flex rounded-md border border-zinc-200 bg-white p-0.5 dark:border-zinc-800 dark:bg-zinc-900/50">
              {UNITS.map((u) => (
                <button
                  key={u}
                  className={`rounded px-2 py-0.5 text-xs font-mono transition-colors ${
                    unit === u
                      ? "bg-emerald-600 text-white"
                      : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200"
                  }`}
                  onClick={() => setUnit(u)}
                >
                  {u}
                </button>
              ))}
            </div>
          </div>
        </div>
      </nav>
      {tab === "wsl" && <WslPage />}
      {tab === "windows" && <WindowsPage />}
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <UnitProvider>
        <Shell />
      </UnitProvider>
    </ThemeProvider>
  );
}
