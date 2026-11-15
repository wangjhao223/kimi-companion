import { useQuery } from "@tanstack/react-query";
import { getCodexStatus, getCodexSummary, getWinKimiSummary } from "../../lib/api";
import SummaryCards from "../stats/SummaryCards";
import TrendChart from "../stats/TrendChart";
import RateChart from "../stats/RateChart";
import SpeedCard from "../stats/SpeedCard";
import CostSummaryCard from "../stats/CostSummaryCard";
import CostCard from "../stats/CostCard";
import ModelShareBar from "../stats/ModelShareBar";
import HeatmapSection from "../heatmap/HeatmapSection";
import CodexStatusCard from "../codex/CodexStatusCard";
import CodexMissingRow from "../CodexMissingRow";
import type { Tool } from "../ToolSelect";
import WinKimiPanel from "./WinKimiPanel";
import { useModuleVisibility } from "../../lib/moduleVisibility";

export default function WindowsPage({ tool }: { tool: Tool }) {
  const { isVisible } = useModuleVisibility();

  const summaryQuery = useQuery({
    queryKey: ["stats-summary", "kimi-win"],
    queryFn: getWinKimiSummary,
    refetchInterval: 10_000,
    enabled: tool === "kimi-code",
  });
  const summary = summaryQuery.data;

  // 自动扫描 Windows 本地的 Codex 数据目录，装好后下一轮轮询（10s）即自动接入。
  // 常驻页面级：未选中 Codex 时也继续探测，切过去立即有状态。
  const codexStatusQuery = useQuery({
    queryKey: ["codex-status"],
    queryFn: getCodexStatus,
    refetchInterval: 10_000,
  });
  const codexFound = codexStatusQuery.data?.dir_found === true;

  const codexSummaryQuery = useQuery({
    queryKey: ["stats-summary", "codex"],
    queryFn: getCodexSummary,
    refetchInterval: 10_000,
    enabled: tool === "codex" && codexFound,
  });
  const codexSummary = codexSummaryQuery.data;

  // 模块显隐：整行模块全部隐藏时不渲染该行，避免残留空网格与间距
  const controlVisible = isVisible("control");
  const summaryVisible = isVisible("summary");
  const ratesVisible = isVisible("rates");
  const costSummaryVisible = isVisible("costSummary");
  const heatmapVisible = isVisible("heatmap");
  const speedVisible = isVisible("speed");
  const trendVisible = isVisible("trend");
  const costDetailVisible = isVisible("costDetail");
  const modelShareVisible = isVisible("modelShare");

  const controlRowVisible = controlVisible || summaryVisible;
  const ratesRowVisible = ratesVisible || costSummaryVisible;
  const heatRowVisible = heatmapVisible || speedVisible;
  const detailRowVisible = trendVisible || costDetailVisible || modelShareVisible;
  const rowsVisible =
    controlRowVisible || ratesRowVisible || heatRowVisible || detailRowVisible;

  // 首个可见行不加上边距，后续行保持 mt-2 的行间距
  const rowGap = (hasRowAbove: boolean) => (hasRowAbove ? "mt-2 " : "");

  return (
    <div className="mx-auto w-full max-w-5xl px-2 pb-3">
      {tool === "kimi-code" ? (
        /*
          统一单元网格（1 个基本单元 = 一张 token 计数小卡）：
          - 第一行（任何宽度）：总控面板 1/3 + token 计数 2/3
          - 速率图 2×1（近 60 分钟 RPM/TPM）· 费用估算（小）1×1（任何宽度并排）
          - 年度热力图 2×1 · 输出速度 1×1（文字小卡）（任何宽度并排）
          - 趋势图 3×1 通栏
          - 费用估算（细）2×2 · 按模型占比 1×2
          桌面端无配额/账本同步模块（那两个是 WSL kimi web 特有）。
        */
        <>
          {controlRowVisible && (
            <div className="grid grid-cols-3 items-stretch gap-x-1 gap-y-2">
              {controlVisible && (
                <div className="min-w-0 col-span-1">
                  <WinKimiPanel />
                </div>
              )}
              {summaryVisible && (
                <div className="min-w-0 col-span-2">
                  <SummaryCards summary={summary} source="kimi-win" />
                </div>
              )}
            </div>
          )}
          {ratesRowVisible && (
            <div className={`${rowGap(controlRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
              {ratesVisible && (
                <div className="min-w-0 col-span-2">
                  <RateChart source="kimi-win" />
                </div>
              )}
              {costSummaryVisible && (
                <div className="min-w-0 col-span-1">
                  <CostSummaryCard summary={summary} source="kimi-win" />
                </div>
              )}
            </div>
          )}
          {heatRowVisible && (
            <div className={`${rowGap(controlRowVisible || ratesRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
              {heatmapVisible && (
                <div className="min-w-0 col-span-2">
                  <HeatmapSection source="kimi-win" />
                </div>
              )}
              {speedVisible && (
                <div className="min-w-0 col-span-1">
                  <SpeedCard source="kimi-win" />
                </div>
              )}
            </div>
          )}
          {detailRowVisible && (
            <div className={`${rowGap(controlRowVisible || ratesRowVisible || heatRowVisible)}grid grid-cols-1 items-stretch gap-x-1 gap-y-2 md:grid-cols-3`}>
              {trendVisible && (
                <div className="min-w-0 md:col-span-3">
                  <TrendChart source="kimi-win" />
                </div>
              )}
              {costDetailVisible && (
                <div className="min-w-0 md:col-span-2 md:row-span-2">
                  <CostCard summary={summary} source="kimi-win" />
                </div>
              )}
              {modelShareVisible && (
                <div className="min-w-0 md:col-span-1 md:row-span-2">
                  <ModelShareBar summary={summary} />
                </div>
              )}
            </div>
          )}
        </>
      ) : !codexFound ? (
        codexStatusQuery.data ? <CodexMissingRow side="Windows 侧" /> : null
      ) : rowsVisible ? (
        <div>
          {/* 网格同 Kimi Code 区块 */}
          {controlRowVisible && (
            <div className="grid grid-cols-1 items-stretch gap-x-1 gap-y-2 md:grid-cols-3">
              {controlVisible && (
                <div className="min-w-0 md:col-span-1 md:row-span-2">
                  <CodexStatusCard source="codex" />
                </div>
              )}
              {summaryVisible && (
                <div className="min-w-0 md:col-span-2 md:row-span-2">
                  <SummaryCards summary={codexSummary} source="codex" />
                </div>
              )}
            </div>
          )}
          {ratesRowVisible && (
            <div className={`${rowGap(controlRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
              {ratesVisible && (
                <div className="min-w-0 col-span-2">
                  <RateChart source="codex" />
                </div>
              )}
              {costSummaryVisible && (
                <div className="min-w-0 col-span-1">
                  <CostSummaryCard summary={codexSummary} source="codex" />
                </div>
              )}
            </div>
          )}
          {heatRowVisible && (
            <div className={`${rowGap(controlRowVisible || ratesRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
              {heatmapVisible && (
                <div className="min-w-0 col-span-2">
                  <HeatmapSection source="codex" />
                </div>
              )}
              {speedVisible && (
                <div className="min-w-0 col-span-1">
                  <SpeedCard source="codex" />
                </div>
              )}
            </div>
          )}
          {detailRowVisible && (
            <div className={`${rowGap(controlRowVisible || ratesRowVisible || heatRowVisible)}grid grid-cols-1 items-stretch gap-x-1 gap-y-2 md:grid-cols-3`}>
              {trendVisible && (
                <div className="min-w-0 md:col-span-3">
                  <TrendChart source="codex" />
                </div>
              )}
              {costDetailVisible && (
                <div className="min-w-0 md:col-span-2 md:row-span-2">
                  <CostCard summary={codexSummary} source="codex" />
                </div>
              )}
              {modelShareVisible && (
                <div className="min-w-0 md:col-span-1 md:row-span-2">
                  <ModelShareBar summary={codexSummary} />
                </div>
              )}
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
}
