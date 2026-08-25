import { useQuery } from "@tanstack/react-query";
import { getLedgerStatus } from "../../lib/api";

/** 账本同步状态（总记录数 / 最近同步时间）。同步线程 30s 一轮，10s 轮询足够。 */
export default function LedgerSyncCard() {
  const ledgerQuery = useQuery({
    queryKey: ["ledger-status"],
    queryFn: getLedgerStatus,
    refetchInterval: 10_000,
  });
  const ledger = ledgerQuery.data;

  return (
    <section className="rounded-xl border border-zinc-800/60 bg-zinc-900/50 p-3.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <div className="flex items-center justify-between">
        <span className="text-sm text-zinc-300">
          账本同步 ·{" "}
          <span className="font-mono text-zinc-100">
            {ledger ? ledger.total_records : "—"}
          </span>{" "}
          条记录
        </span>
        <span className="text-xs text-zinc-500">
          {ledger?.last_sync_at_ms
            ? `最近同步 ${new Date(ledger.last_sync_at_ms).toLocaleTimeString()}`
            : "尚未同步"}
        </span>
      </div>
      {ledger?.last_error && (
        <p className="mt-2 rounded-md border border-red-900/60 bg-red-950/40 px-3 py-2 text-sm text-red-300">
          同步出错：{ledger.last_error}
        </p>
      )}
    </section>
  );
}
