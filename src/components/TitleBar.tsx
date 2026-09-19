import { getCurrentWindow } from "@tauri-apps/api/window";

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
    <header className="flex h-9 shrink-0 items-center border-b border-zinc-200 bg-white/80 dark:border-zinc-800/60 dark:bg-zinc-950/80">
      <div className="flex items-center gap-2 pl-3">
        <span className="h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]" />
        <span className="text-sm font-medium tracking-wide text-zinc-700 dark:text-zinc-300">
          Kimi Companion
        </span>
      </div>
      {/* 拖拽区：占据标题栏中间剩余空间 */}
      <div data-tauri-drag-region className="h-full flex-1" />
      <div className="flex h-full items-stretch">
        <button
          className="flex w-11 items-center justify-center text-zinc-500 transition-colors hover:bg-zinc-200 hover:text-zinc-800 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          onClick={minimize}
          title="最小化"
        >
          <MinimizeIcon />
        </button>
        <button
          className="flex w-11 items-center justify-center text-zinc-500 transition-colors hover:bg-red-600 hover:text-white"
          onClick={close}
          title="关闭"
        >
          <CloseIcon />
        </button>
      </div>
    </header>
  );
}
