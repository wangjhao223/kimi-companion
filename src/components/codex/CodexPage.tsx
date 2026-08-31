import { useQuery } from "@tanstack/react-query";
import { getCodexSummary } from "../../lib/api";
import SummaryCards from "../stats/SummaryCards";
import TrendChart from "../stats/TrendChart";
import CostSummaryCard from "../stats/CostSummaryCard";
import CostCard from "../stats/CostCard";
import ModelShareBar from "../stats/ModelShareBar";
import HeatmapSection from "../heatmap/HeatmapSection";
import CodexStatusCard from "./CodexStatusCard";

export default function CodexPage() {
  const summaryQuery = useQuery({
    queryKey: ["stats-summary", "codex"],
    queryFn: getCodexSummary,
    refetchInterval: 10_000,
  });
  const summary = summaryQuery.data;

  return (
    <div className="mx-auto w-full max-w-5xl px-5 py-4">
      {/*
        与首页同一套单元网格（1 个基本单元 = 一张 token 计数小卡）：
        - 数据状态卡 1×2 · 用量汇总 2×2
        - 趋势图 2×1 · 费用估算（小）1×1
        - 年度热力图 3×1
        - 费用估算（细）2×2 · 按模型占比 1×2
      */}
      <div className="grid grid-cols-1 items-stretch gap-2.5 md:grid-cols-3">
        <div className="min-w-0 md:col-span-1 md:row-span-2">
          <CodexStatusCard />
        </div>
        <div className="min-w-0 md:col-span-2 md:row-span-2">
          <SummaryCards summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-2">
          <TrendChart source="codex" />
        </div>
        <div className="min-w-0 md:col-span-1">
          <CostSummaryCard summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-3">
          <HeatmapSection source="codex" />
        </div>
        <div className="min-w-0 md:col-span-2 md:row-span-2">
          <CostCard summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-1 md:row-span-2">
          <ModelShareBar summary={summary} />
        </div>
      </div>
    </div>
  );
}
