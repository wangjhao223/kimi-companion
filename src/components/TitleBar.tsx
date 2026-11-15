import { useCallback, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import SettingsDialog from "./settings/SettingsDialog";
import { useTheme } from "../lib/theme";
import type { ThemeMode } from "../lib/theme";
import { useUnit } from "../lib/unit";
import type { Unit } from "../lib/format";

const UNITS: { key: Unit; label: string }[] = [
  { key: "k", label: "k" },
  { key: "M", label: "M" },
  { key: "B", label: "B" },
  { key: "auto", label: "自适应" },
];

const THEME_OPTIONS: { key: ThemeMode; label: string }[] = [
  { key: "light", label: "亮" },
  { key: "dark", label: "暗" },
  { key: "system", label: "跟随" },
];

function GearIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.6" />
    </svg>
  );
}

function MinimizeIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path d="M2 6h8" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path
        d="M3 3l6 6M9 3l-6 6"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default function TitleBar() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const closeSettings = useCallback(() => setSettingsOpen(false), []);
  const { mode, setMode } = useTheme();
  const { unit, setUnit } = useUnit();

  // 纯浏览器预览（无 Tauri 运行时）时调用会失败，静默忽略即可
  const minimize = () => {
    getCurrentWindow()
      .minimize()
      .catch(() => {});
  };
  const close = () => {
    getCurrentWindow()
      .close()
      .catch(() => {});
  };

  return (
    <header className="sticky top-0 z-30 flex h-9 shrink-0 items-center border-b border-zinc-200 bg-white/80 backdrop-blur dark:border-zinc-800/60 dark:bg-zinc-950/80">
      <div className="flex min-w-0 items-center gap-2 pl-3">
        <span className="h-2 w-2 shrink-0 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]" />
        <span className="truncate text-sm font-medium tracking-wide text-zinc-700 dark:text-zinc-300">
          Kimi Companion
        </span>
      </div>
      {/* 拖拽区：占据标题栏中间剩余空间，窄窗口下也不会被两侧控件挤没 */}
      <div data-tauri-drag-region className="h-full min-w-4 flex-1" />
      <div className="flex h-full shrink-0 items-stretch">
        {/* 主题与单位设置：窄窗口下隐藏文字标签，仅保留按钮组 */}
        <div className="flex items-center gap-1.5 px-1.5">
          <span className="hidden text-xs text-zinc-400 dark:text-zinc-600 lg:inline">
            主题
          </span>
          <div
            role="group"
            aria-label="主题"
            className="flex shrink-0 rounded-md border border-zinc-200 bg-white p-0.5 dark:border-zinc-800 dark:bg-zinc-900/50"
          >
            {THEME_OPTIONS.map((t) => (
              <button
                key={t.key}
                className={`rounded px-1.5 py-0.5 text-xs transition-colors ${
                  mode === t.key
                    ? "bg-emerald-600 text-white"
                    : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200"
                }`}
                onClick={() => setMode(t.key)}
                aria-pressed={mode === t.key}
                title={`主题：${t.label}`}
              >
                {t.label}
              </button>
            ))}
          </div>
          <span className="hidden text-xs text-zinc-400 dark:text-zinc-600 lg:inline">
            单位
          </span>
          <div
            role="group"
            aria-label="单位"
            className="flex shrink-0 rounded-md border border-zinc-200 bg-white p-0.5 dark:border-zinc-800 dark:bg-zinc-900/50"
          >
            {UNITS.map((u) => (
              <button
                key={u.key}
                className={`rounded px-1.5 py-0.5 text-xs font-mono transition-colors ${
                  unit === u.key
                    ? "bg-emerald-600 text-white"
                    : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200"
                }`}
                onClick={() => setUnit(u.key)}
                aria-pressed={unit === u.key}
                title={`单位：${u.label}`}
              >
                {u.label}
              </button>
            ))}
          </div>
        </div>
        <button
          className="flex shrink-0 items-center gap-1 px-2.5 text-xs text-zinc-500 transition-colors hover:bg-zinc-200 hover:text-zinc-800 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          onClick={() => setSettingsOpen(true)}
          title="设置"
          aria-label="设置"
          aria-haspopup="dialog"
          aria-expanded={settingsOpen}
        >
          <GearIcon />
          <span className="hidden sm:inline">设置</span>
        </button>
        <button
          className="flex w-11 shrink-0 items-center justify-center text-zinc-500 transition-colors hover:bg-zinc-200 hover:text-zinc-800 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          onClick={minimize}
          title="最小化"
        >
          <MinimizeIcon />
        </button>
        <button
          className="flex w-11 shrink-0 items-center justify-center text-zinc-500 transition-colors hover:bg-red-600 hover:text-white"
          onClick={close}
          title="关闭"
        >
          <CloseIcon />
        </button>
      </div>
      <SettingsDialog open={settingsOpen} onClose={closeSettings} />
    </header>
  );
}