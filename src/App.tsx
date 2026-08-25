import { useState } from "react";
import HomePage from "./components/home/HomePage";
import HeatmapPage from "./components/heatmap/HeatmapPage";
import TitleBar from "./components/TitleBar";
import { UnitProvider, useUnit } from "./lib/unit";
import type { Unit } from "./lib/format";

type Tab = "home" | "heatmap";

const TABS: { key: Tab; label: string }[] = [
  { key: "home", label: "首页" },
  { key: "heatmap", label: "详情" },
];

const UNITS: Unit[] = ["k", "M"];

function Shell() {
  const [tab, setTab] = useState<Tab>("home");
  const { unit, setUnit } = useUnit();

  return (
    <div className="flex min-h-screen flex-col">
      <TitleBar />
      <nav className="sticky top-0 z-10 border-b border-zinc-800/60 bg-zinc-950/80 backdrop-blur">
        <div className="mx-auto flex max-w-5xl items-center gap-1 px-5 py-1.5">
          {TABS.map((t) => (
            <button
              key={t.key}
              className={`rounded-md px-3 py-1 text-[13px] font-medium transition-colors ${
                tab === t.key
                  ? "bg-zinc-800 text-emerald-300"
                  : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200"
              }`}
              onClick={() => setTab(t.key)}
            >
              {t.label}
            </button>
          ))}
          <div className="ml-auto flex items-center gap-2">
            <span className="text-xs text-zinc-600">单位</span>
            <div className="flex rounded-md border border-zinc-800 bg-zinc-900/50 p-0.5">
              {UNITS.map((u) => (
                <button
                  key={u}
                  className={`rounded px-2 py-0.5 text-xs font-mono transition-colors ${
                    unit === u
                      ? "bg-emerald-600 text-white"
                      : "text-zinc-400 hover:text-zinc-200"
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
      {tab === "home" && <HomePage />}
      {tab === "heatmap" && <HeatmapPage />}
    </div>
  );
}

export default function App() {
  return (
    <UnitProvider>
      <Shell />
    </UnitProvider>
  );
}
