import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import {
  MODULE_DEFINITIONS,
  useModuleVisibility,
} from "../../lib/moduleVisibility";

type ModuleDefinition = (typeof MODULE_DEFINITIONS)[number];

/** 按 group 归并模块，保留定义顺序；没有 group 的模块直接平铺。 */
const GROUPS = (() => {
  const map = new Map<string, ModuleDefinition[]>();
  for (const definition of MODULE_DEFINITIONS) {
    const key = definition.group ?? "";
    const list = map.get(key);
    if (list) {
      list.push(definition);
    } else {
      map.set(key, [definition]);
    }
  }
  return [...map.entries()].map(([group, modules]) => ({ group, modules }));
})();

function CloseIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <path
        d="M3.5 3.5l7 7M10.5 3.5l-7 7"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </svg>
  );
}

/** 模块显示设置弹窗：勾选控制各模块显隐，支持全显、全部隐藏与关闭。 */
export default function SettingsDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { isVisible, setVisible, setAll } = useModuleVisibility();
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const previousActive = document.activeElement as HTMLElement | null;
    closeButtonRef.current?.focus();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      onClose();
    };
    document.addEventListener("keydown", onKeyDown);

    // 弹窗打开期间锁定页面滚动，避免滚动穿透
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = previousOverflow;
      previousActive?.focus();
    };
  }, [open, onClose]);

  if (!open) return null;

  const visibleCount = MODULE_DEFINITIONS.filter((definition) =>
    isVisible(definition.id)
  ).length;
  const total = MODULE_DEFINITIONS.length;
  const allVisible = visibleCount === total;
  const allHidden = visibleCount === 0;

  const secondaryButton =
    "rounded-md border border-zinc-300 px-2 py-1 text-xs text-zinc-600 transition-colors hover:bg-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/50 disabled:cursor-not-allowed disabled:opacity-40 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800";

  // TitleBar 的 backdrop-blur（backdrop-filter）会为 fixed 后代建立包含块，直接渲染会使弹窗偏离视口中心；通过 Portal 挂载到 body 以视口为定位基准。
  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-dialog-title"
        className="flex max-h-[85vh] w-full max-w-md flex-col overflow-hidden rounded-xl border border-zinc-200 bg-white shadow-2xl [color-scheme:light] dark:border-zinc-800 dark:bg-zinc-900 dark:[color-scheme:dark]"
      >
        <div className="flex items-start justify-between gap-3 border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
          <div>
            <h2
              id="settings-dialog-title"
              className="text-sm font-semibold text-zinc-800 dark:text-zinc-100"
            >
              设置
            </h2>
            <p className="mt-0.5 text-xs text-zinc-500 dark:text-zinc-400">
              选择主界面要显示的模块
            </p>
          </div>
          <button
            ref={closeButtonRef}
            type="button"
            onClick={onClose}
            title="关闭设置"
            aria-label="关闭设置"
            className="-mr-1 rounded-md p-1 text-zinc-400 transition-colors hover:bg-zinc-100 hover:text-zinc-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/50 dark:text-zinc-500 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          >
            <CloseIcon />
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-2 py-2">
          {GROUPS.map(({ group, modules }) => (
            <section key={group || "ungrouped"} className="mb-1 last:mb-0">
              {group && (
                <h3 className="px-2 pb-1 pt-2 text-xs font-medium text-zinc-400 dark:text-zinc-500">
                  {group}
                </h3>
              )}
              <div className="space-y-0.5">
                {modules.map((definition) => (
                  <label
                    key={definition.id}
                    className="flex cursor-pointer items-start gap-2.5 rounded-md px-2 py-1.5 transition-colors hover:bg-zinc-50 dark:hover:bg-zinc-800/60"
                  >
                    <input
                      type="checkbox"
                      checked={isVisible(definition.id)}
                      onChange={(event) =>
                        setVisible(definition.id, event.target.checked)
                      }
                      className="mt-0.5 h-4 w-4 shrink-0 cursor-pointer accent-emerald-600"
                    />
                    <span className="min-w-0">
                      <span className="block text-sm text-zinc-700 dark:text-zinc-200">
                        {definition.label}
                      </span>
                      {definition.description && (
                        <span className="mt-0.5 block text-xs leading-relaxed text-zinc-500 dark:text-zinc-400">
                          {definition.description}
                        </span>
                      )}
                    </span>
                  </label>
                ))}
              </div>
            </section>
          ))}
        </div>

        <div className="flex flex-wrap items-center gap-2 border-t border-zinc-200 px-4 py-3 dark:border-zinc-800">
          <span className="mr-auto text-xs text-zinc-400 dark:text-zinc-500">
            {visibleCount} / {total} 个模块已显示
          </span>
          <button
            type="button"
            onClick={() => setAll(true)}
            disabled={allVisible}
            className={secondaryButton}
          >
            全部显示
          </button>
          <button
            type="button"
            onClick={() => setAll(false)}
            disabled={allHidden}
            className={secondaryButton}
          >
            全部隐藏
          </button>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md bg-emerald-600 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-emerald-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/50"
          >
            关闭
          </button>
        </div>
      </div>
    </div>,
    document.body
  );
}