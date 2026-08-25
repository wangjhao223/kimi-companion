import { useQuery } from "@tanstack/react-query";
import { getStatsSummary } from "../../lib/api";
import LaunchPanel from "../launch/LaunchPanel";
import SummaryCards from "../stats/SummaryCards";
import TrendChart from "../stats/TrendChart";
import CostSummaryCard from "../stats/CostSummaryCard";

export default function HomePage() {
  const summaryQuery = useQuery({
    queryKey: ["stats-summary"],
    queryFn: getStatsSummary,
    refetchInterval: 30_000,
  });
  const summary = summaryQuery.data;

  return (
    <div className="mx-auto w-full max-w-5xl px-5 py-4">
      {/* 上半部分：左启动控制，右用量汇总；宽度不够时右列换行堆叠 */}
      <div className="flex flex-wrap items-start gap-2.5">
        <div className="min-w-0 flex-1 basis-[340px]">
          <LaunchPanel />
        </div>
        <div className="min-w-0 flex-1 basis-[360px]">
          <SummaryCards summary={summary} />
        </div>
      </div>

      {/* 下半部分：趋势图 + 费用小总结（卡片尺寸与上方汇总卡一致） */}
      <div className="mt-2.5 flex flex-wrap items-start gap-2.5">
        <div className="min-w-0 flex-1 basis-[420px]">
          <TrendChart />
        </div>
        <div className="min-w-0 flex-none basis-[240px]">
          <CostSummaryCard summary={summary} />
        </div>
      </div>
    </div>
  );
}
