import { useQuery } from "@tanstack/react-query";
import { getStatsSummary } from "../../lib/api";
import LaunchPanel from "../launch/LaunchPanel";
import LedgerSyncCard from "../launch/LedgerSyncCard";
import SummaryCards from "../stats/SummaryCards";
import TrendChart from "../stats/TrendChart";
import CostSummaryCard from "../stats/CostSummaryCard";
import CostCard from "../stats/CostCard";
import QuotaCard from "../stats/QuotaCard";
import ModelShareBar from "../stats/ModelShareBar";
import HeatmapSection from "../heatmap/HeatmapSection";

export default function HomePage() {
  const summaryQuery = useQuery({
    queryKey: ["stats-summary", "kimi"],
    queryFn: getStatsSummary,
    refetchInterval: 10_000,
  });
  const summary = summaryQuery.data;

  return (
    <div className="mx-auto w-full max-w-5xl px-5 py-4">
      {/*
        统一网格排版：以一张 token 计数小卡为 1 个基本单元（1 列 × 1 行），
        所有模块的宽高都是基本单元的整数倍，行列自然对齐：
        - 总控面板     1×2（一列宽、上下两个单元高）
        - 用量汇总     2×2（内部 2×2 小卡，每卡恰好 1 个单元）
        - 趋势图       2×1 · 费用估算（小）1×1
        - 年度热力图   3×1（整行）
        - 费用估算（细）2×2 · 套餐配额 1×1 · 按模型占比 1×1
        - 账本同步     3×1（整行收尾）
        窗口过窄（<md）时退化为单列纵向堆叠。
      */}
      <div className="grid grid-cols-1 items-stretch gap-2.5 md:grid-cols-3">
        <div className="min-w-0 md:col-span-1 md:row-span-2">
          <LaunchPanel />
        </div>
        <div className="min-w-0 md:col-span-2 md:row-span-2">
          <SummaryCards summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-2">
          <TrendChart source="kimi" />
        </div>
        <div className="min-w-0 md:col-span-1">
          <CostSummaryCard summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-3">
          <HeatmapSection source="kimi" />
        </div>
        <div className="min-w-0 md:col-span-2 md:row-span-2">
          <CostCard summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-1">
          <QuotaCard />
        </div>
        <div className="min-w-0 md:col-span-1">
          <ModelShareBar summary={summary} />
        </div>
        <div className="min-w-0 md:col-span-3">
          <LedgerSyncCard />
        </div>
      </div>
    </div>
  );
}
