import { useQuery } from "@tanstack/react-query";
import {
  getStatsSummary,
  getWslCodexStatus,
  getWslCodexSummary,
  listDistros,
} from "../../lib/api";
import LaunchPanel from "../launch/LaunchPanel";
import LedgerSyncCard from "../launch/LedgerSyncCard";
import UpdateCard from "../launch/UpdateCard";
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
import { useModuleVisibility } from "../../lib/moduleVisibility";

export default function WslPage({ tool }: { tool: Tool }) {
  // 模块显示/隐藏由顶栏设置控制，默认全部显示
  const { isVisible } = useModuleVisibility();
  const showControl = isVisible("control");
  const showSummary = isVisible("summary");
  const showRates = isVisible("rates");
  const showCostSummary = isVisible("costSummary");
  const showHeatmap = isVisible("heatmap");
  const showSpeed = isVisible("speed");
  const showTrend = isVisible("trend");
  const showCostDetail = isVisible("costDetail");
  const showModelShare = isVisible("modelShare");
  const showUpdate = isVisible("update");
  const showLedger = isVisible("ledger");
  // 整行模块全部隐藏时不渲染该行，避免残留空网格与间距
  const controlRowVisible = showControl || showSummary;
  const ratesRowVisible = showRates || showCostSummary;
  const heatRowVisible = showHeatmap || showSpeed;
  const kimiDetailRowVisible =
    showTrend || showCostDetail || showUpdate || showModelShare || showLedger;
  const codexDetailRowVisible = showTrend || showCostDetail || showModelShare;
  // Codex 区块不含更新卡与账本同步，单独判断是否还有可见模块
  const codexHasVisible =
    controlRowVisible || ratesRowVisible || heatRowVisible || codexDetailRowVisible;

  // 首个可见行不加上边距，后续行保持 mt-2 的行间距
  const rowGap = (hasRowAbove: boolean) => (hasRowAbove ? "mt-2 " : "");

  // 程序选择由顶栏统一管理，本页只按传入的 tool 展示对应数据
  const summaryQuery = useQuery({
    queryKey: ["stats-summary", "kimi"],
    queryFn: getStatsSummary,
    refetchInterval: 10_000,
    enabled: tool === "kimi-code",
  });
  const summary = summaryQuery.data;

  // 与 LaunchPanel/UpdateCard 共用 ["distros"] 缓存，取第一个发行版名供汇总/费用卡识别模型来源
  const distrosQuery = useQuery({
    queryKey: ["distros"],
    queryFn: listDistros,
    retry: 1,
  });
  const distro = distrosQuery.data?.[0];

  // 自动扫描 WSL 内的 Codex 数据目录，装好后下一轮轮询（10s）即自动接入。
  // 常驻页面级：未选中 Codex 时也继续探测，切过去立即有状态。
  const codexStatusQuery = useQuery({
    queryKey: ["wsl-codex-status"],
    queryFn: getWslCodexStatus,
    refetchInterval: 10_000,
  });
  const codexFound = codexStatusQuery.data?.dir_found === true;

  const codexSummaryQuery = useQuery({
    queryKey: ["stats-summary", "wsl-codex"],
    queryFn: getWslCodexSummary,
    refetchInterval: 10_000,
    enabled: tool === "codex" && codexFound,
  });
  const codexSummary = codexSummaryQuery.data;

  return (
    <div className="mx-auto w-full max-w-5xl px-2 pb-3">
      {tool === "kimi-code" ? (
        /*
          统一网格排版：以一张 token 计数小卡为 1 个基本单元（1 列 × 1 行），
          所有模块的宽高都是基本单元的整数倍，行列自然对齐：
          - 第一行（任何宽度）：总控面板 1/3 + token 计数 2/3
          - 速率图       2×1（近 60 分钟 RPM/TPM）· 费用估算（小）1×1（任何宽度并排）
          - 年度热力图   2×1 · 输出速度 1×1（文字小卡）（任何宽度并排）
          - 趋势图       3×1（整行）
          - 费用估算（细）2×2 · Kimi Code 更新 1×1 · 按模型占比 1×1
          - 账本同步     3×1（整行收尾）
          除前两对任何宽度并排外，其余模块在窗口过窄（<md）时退化为单列纵向堆叠。
          设置中隐藏的模块不渲染；一行全部隐藏时，该行网格与间距一并省略。
        */
        <>
          {controlRowVisible ? (
            <div className="grid grid-cols-3 items-stretch gap-x-1 gap-y-2">
              {showControl ? (
                <div className="min-w-0 col-span-1">
                  <LaunchPanel />
                </div>
              ) : null}
              {showSummary ? (
                <div className="min-w-0 col-span-2">
                  <SummaryCards summary={summary} source="kimi" distro={distro} />
                </div>
              ) : null}
            </div>
          ) : null}
          {ratesRowVisible ? (
            <div className={`${rowGap(controlRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
              {showRates ? (
                <div className="min-w-0 col-span-2">
                  <RateChart source="kimi" />
                </div>
              ) : null}
              {showCostSummary ? (
                <div className="min-w-0 col-span-1">
                  <CostSummaryCard summary={summary} source="kimi" distro={distro} />
                </div>
              ) : null}
            </div>
          ) : null}
          {heatRowVisible ? (
            <div className={`${rowGap(controlRowVisible || ratesRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
              {showHeatmap ? (
                <div className="min-w-0 col-span-2">
                  <HeatmapSection source="kimi" />
                </div>
              ) : null}
              {showSpeed ? (
                <div className="min-w-0 col-span-1">
                  <SpeedCard source="kimi" />
                </div>
              ) : null}
            </div>
          ) : null}
          {kimiDetailRowVisible ? (
            <div className={`${rowGap(controlRowVisible || ratesRowVisible || heatRowVisible)}grid grid-cols-1 items-stretch gap-x-1 gap-y-2 md:grid-cols-3`}>
              {showTrend ? (
                <div className="min-w-0 md:col-span-3">
                  <TrendChart source="kimi" />
                </div>
              ) : null}
              {showCostDetail ? (
                <div className="min-w-0 md:col-span-2 md:row-span-2">
                  <CostCard summary={summary} source="kimi" distro={distro} />
                </div>
              ) : null}
              {showUpdate ? (
                <div className="min-w-0 md:col-span-1">
                  <UpdateCard />
                </div>
              ) : null}
              {showModelShare ? (
                <div className="min-w-0 md:col-span-1">
                  <ModelShareBar summary={summary} />
                </div>
              ) : null}
              {showLedger ? (
                <div className="min-w-0 md:col-span-3">
                  <LedgerSyncCard />
                </div>
              ) : null}
            </div>
          ) : null}
        </>
      ) : codexFound ? (
        codexHasVisible ? (
          /*
            与 Kimi Code 区块同一套单元网格：
            - 数据状态卡 1×2 · 用量汇总 2×2
            - 速率图 2×1（近 60 分钟 RPM/TPM）· 费用估算（小）1×1（任何宽度并排）
            - 年度热力图 2×1 · 输出速度 1×1（文字小卡）（任何宽度并排）
            - 趋势图 3×1（整行）
            - 费用估算（细）2×2 · 按模型占比 1×2
            设置中隐藏的模块不渲染；一行全部隐藏时，该行网格与间距一并省略。
          */
          <div>
            {controlRowVisible ? (
              <div className="grid grid-cols-1 items-stretch gap-x-1 gap-y-2 md:grid-cols-3">
                {showControl ? (
                  <div className="min-w-0 md:col-span-1 md:row-span-2">
                    <CodexStatusCard source="wsl-codex" />
                  </div>
                ) : null}
                {showSummary ? (
                  <div className="min-w-0 md:col-span-2 md:row-span-2">
                    <SummaryCards summary={codexSummary} source="wsl-codex" />
                  </div>
                ) : null}
              </div>
            ) : null}
            {ratesRowVisible ? (
              <div className={`${rowGap(controlRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
                {showRates ? (
                  <div className="min-w-0 col-span-2">
                    <RateChart source="wsl-codex" />
                  </div>
                ) : null}
                {showCostSummary ? (
                  <div className="min-w-0 col-span-1">
                    <CostSummaryCard summary={codexSummary} source="wsl-codex" />
                  </div>
                ) : null}
              </div>
            ) : null}
            {heatRowVisible ? (
              <div className={`${rowGap(controlRowVisible || ratesRowVisible)}grid grid-cols-3 items-stretch gap-x-1 gap-y-2`}>
                {showHeatmap ? (
                  <div className="min-w-0 col-span-2">
                    <HeatmapSection source="wsl-codex" />
                  </div>
                ) : null}
                {showSpeed ? (
                  <div className="min-w-0 col-span-1">
                    <SpeedCard source="wsl-codex" />
                  </div>
                ) : null}
              </div>
            ) : null}
            {codexDetailRowVisible ? (
              <div className={`${rowGap(controlRowVisible || ratesRowVisible || heatRowVisible)}grid grid-cols-1 items-stretch gap-x-1 gap-y-2 md:grid-cols-3`}>
                {showTrend ? (
                  <div className="min-w-0 md:col-span-3">
                    <TrendChart source="wsl-codex" />
                  </div>
                ) : null}
                {showCostDetail ? (
                  <div className="min-w-0 md:col-span-2 md:row-span-2">
                    <CostCard summary={codexSummary} source="wsl-codex" />
                  </div>
                ) : null}
                {showModelShare ? (
                  <div className="min-w-0 md:col-span-1 md:row-span-2">
                    <ModelShareBar summary={codexSummary} />
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null
      ) : codexStatusQuery.data ? (
        <CodexMissingRow side="WSL 侧" />
      ) : null}
    </div>
  );
}
