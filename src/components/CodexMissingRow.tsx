/** Codex 未检测到时的整行（3×1）虚线占位；安装后下一轮状态轮询即自动接入。 */
export default function CodexMissingRow({ side }: { side: string }) {
  return (
    <div className="flex items-center justify-center rounded-xl border border-dashed border-zinc-300 py-6 text-sm text-zinc-500 dark:border-zinc-700">
      未检测到 Codex（{side}）· 安装后自动接入
    </div>
  );
}
