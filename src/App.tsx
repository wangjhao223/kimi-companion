import { useState } from "react";
import WslPage from "./components/wsl/WslPage";
import WindowsPage from "./components/windows/WindowsPage";
import TitleBar from "./components/TitleBar";
import ToolSelect, { useToolSelection } from "./components/ToolSelect";
import { UnitProvider } from "./lib/unit";
import { ThemeProvider } from "./lib/theme";
import { ModuleVisibilityProvider } from "./lib/moduleVisibility";

type Tab = "wsl" | "windows";

const TABS: { key: Tab; label: string }[] = [
  { key: "wsl", label: "wsl" },
  { key: "windows", label: "windows" },
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
  // 两个平台各自记忆上次选择的程序（Kimi Code / Codex）
  const [wslTool, setWslTool] = useToolSelection("kimi-companion.tool.wsl");
  const [windowsTool, setWindowsTool] = useToolSelection("kimi-companion.tool.windows");

  const activeTool = tab === "wsl" ? wslTool : windowsTool;
  const selectTool = tab === "wsl" ? setWslTool : setWindowsTool;

  return (
    <div className="flex min-h-screen flex-col">
      <TitleBar />
      <nav className="sticky top-9 z-20 border-b border-zinc-200 bg-white/80 backdrop-blur dark:border-zinc-800/60 dark:bg-zinc-950/80">
        <div className="mx-auto flex max-w-5xl items-center gap-1 px-2 py-1.5">
          {TABS.map((t) => (
            <button
              key={t.key}
              className={`rounded-md px-1.5 py-1 text-[13px] font-medium transition-colors ${
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
            <ToolSelect value={activeTool} onChange={selectTool} />
          </div>
        </div>
      </nav>
      {tab === "wsl" ? <WslPage tool={wslTool} /> : <WindowsPage tool={windowsTool} />}
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <UnitProvider>
        <ModuleVisibilityProvider>
          <Shell />
        </ModuleVisibilityProvider>
      </UnitProvider>
    </ThemeProvider>
  );
}
