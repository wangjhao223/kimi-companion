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
import CostSummaryCard from "../stats/CostSummaryCard";
import CostCard from "../stats/CostCard";
import ModelShareBar from "../stats/ModelShareBar";
import HeatmapSection from "../heatmap/HeatmapSection";
import CodexStatusCard from "../codex/CodexStatusCard";
import CodexMissingRow from "../CodexMissingRow";
import ToolSelect, { useToolSelection } from "../ToolSelect";

export default function WslPage() {
  // 程序选择：Kimi Code / Codex 二选一显示，选择按页面持久化
  const [tool, setTool] = useToolSelection("kimi-companion.tool.wsl");

  const summaryQuery = useQuery({
    queryKey: ["stats-summary", "kimi"],
    queryFn: getStatsSummary,
    refetchInterval: 10_000,
    enabled: tool === "kimi-code",
  });
  const summary = summaryQuery.data;

  // 与 LaunchPanel/UpdateCard 共用 ["distros"] 缓存，取第一个发行版名做副标题
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

  const subtitle =
    tool === "kimi-code"
      ? distro
        ? `WSL · ${distro}`
        : "WSL"
      : "WSL 侧 · 自动扫描";

  return (
    <div className="mx-auto w-full max-w-5xl px-5 py-4">
      <div className="flex items-baseline gap-3">
        <ToolSelect value={tool} onChange={setTool} />
        <span className="text-sm text-zinc-500">{subtitle}</span>
      </div>

      {tool === "kimi-code" ? (
        /*
          统一网格排版：以一张 token 计数小卡为 1 个基本单元（1 列 × 1 行），
          所有模块的宽高都是基本单元的整数倍，行列自然对齐：
          - 总控面板     1×2（一列宽、上下两个单元高）
          - 用量汇总     2×2（内部 2×2 小卡，每卡恰好 1 个单元）
          - 趋势图       2×1 · 费用估算（小）1×1
          - 年度热力图   3×1（整行）
          - 费用估算（细）2×2 · Kimi Code 更新 1×1 · 按模型占比 1×1
          - 账本同步     3×1（整行收尾）
          窗口过窄（<md）时退化为单列纵向堆叠。
        */
        <div className="mt-2.5 grid grid-cols-1 items-stretch gap-2.5 md:grid-cols-3">
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
            <UpdateCard />
          </div>
          <div className="min-w-0 md:col-span-1">
            <ModelShareBar summary={summary} />
          </div>
          <div className="min-w-0 md:col-span-3">
            <LedgerSyncCard />
          </div>
        </div>
      ) : (
        <div className="mt-2.5">
          {codexFound ? (
            /*
              与 Kimi Code 区块同一套单元网格：
              - 数据状态卡 1×2 · 用量汇总 2×2
              - 趋势图 2×1 · 费用估算（小）1×1
              - 年度热力图 3×1
              - 费用估算（细）2×2 · 按模型占比 1×2
            */
            <div className="grid grid-cols-1 items-stretch gap-2.5 md:grid-cols-3">
              <div className="min-w-0 md:col-span-1 md:row-span-2">
                <CodexStatusCard source="wsl-codex" />
              </div>
              <div className="min-w-0 md:col-span-2 md:row-span-2">
                <SummaryCards summary={codexSummary} />
              </div>
              <div className="min-w-0 md:col-span-2">
                <TrendChart source="wsl-codex" />
              </div>
              <div className="min-w-0 md:col-span-1">
                <CostSummaryCard summary={codexSummary} />
              </div>
              <div className="min-w-0 md:col-span-3">
                <HeatmapSection source="wsl-codex" />
              </div>
              <div className="min-w-0 md:col-span-2 md:row-span-2">
                <CostCard summary={codexSummary} />
              </div>
              <div className="min-w-0 md:col-span-1 md:row-span-2">
                <ModelShareBar summary={codexSummary} />
              </div>
            </div>
          ) : codexStatusQuery.data ? (
            <CodexMissingRow side="WSL 侧" />
          ) : null}
        </div>
      )}
    </div>
  );
}
