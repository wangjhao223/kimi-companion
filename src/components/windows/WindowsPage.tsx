import { useQuery } from "@tanstack/react-query";
import { getCodexStatus, getCodexSummary, getWinKimiSummary } from "../../lib/api";
import SummaryCards from "../stats/SummaryCards";
import TrendChart from "../stats/TrendChart";
import CostSummaryCard from "../stats/CostSummaryCard";
import CostCard from "../stats/CostCard";
import ModelShareBar from "../stats/ModelShareBar";
import HeatmapSection from "../heatmap/HeatmapSection";
import CodexStatusCard from "../codex/CodexStatusCard";
import CodexMissingRow from "../CodexMissingRow";
import ToolSelect, { useToolSelection } from "../ToolSelect";
import WinKimiPanel from "./WinKimiPanel";

export default function WindowsPage() {
  // 程序选择：Kimi Code / Codex 二选一显示，选择按页面持久化
  const [tool, setTool] = useToolSelection("kimi-companion.tool.windows");

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

  const subtitle =
    tool === "kimi-code" ? "Windows 桌面端" : "Windows 本地 · 自动扫描";

  return (
    <div className="mx-auto w-full max-w-5xl px-5 py-4">
      <div className="flex items-baseline gap-3">
        <ToolSelect value={tool} onChange={setTool} />
        <span className="text-sm text-zinc-500">{subtitle}</span>
      </div>

      {tool === "kimi-code" ? (
        /*
          统一单元网格（1 个基本单元 = 一张 token 计数小卡）：
          - 左列 1×2：WinKimiPanel（h-full 撑满）
          - 用量汇总 2×2
          - 趋势图 2×1 · 费用估算（小）1×1
          - 年度热力图 3×1
          - 费用估算（细）2×2 · 按模型占比 1×2
          桌面端无配额/账本同步模块（那两个是 WSL kimi web 特有）。
        */
        <div className="mt-2.5 grid grid-cols-1 items-stretch gap-2.5 md:grid-cols-3">
          <div className="min-w-0 md:col-span-1 md:row-span-2">
            <WinKimiPanel />
          </div>
          <div className="min-w-0 md:col-span-2 md:row-span-2">
            <SummaryCards summary={summary} />
          </div>
          <div className="min-w-0 md:col-span-2">
            <TrendChart source="kimi-win" />
          </div>
          <div className="min-w-0 md:col-span-1">
            <CostSummaryCard summary={summary} />
          </div>
          <div className="min-w-0 md:col-span-3">
            <HeatmapSection source="kimi-win" />
          </div>
          <div className="min-w-0 md:col-span-2 md:row-span-2">
            <CostCard summary={summary} />
          </div>
          <div className="min-w-0 md:col-span-1 md:row-span-2">
            <ModelShareBar summary={summary} />
          </div>
        </div>
      ) : (
        <div className="mt-2.5">
          {codexFound ? (
            /* 网格同 Kimi Code 区块 */
            <div className="grid grid-cols-1 items-stretch gap-2.5 md:grid-cols-3">
              <div className="min-w-0 md:col-span-1 md:row-span-2">
                <CodexStatusCard source="codex" />
              </div>
              <div className="min-w-0 md:col-span-2 md:row-span-2">
                <SummaryCards summary={codexSummary} />
              </div>
              <div className="min-w-0 md:col-span-2">
                <TrendChart source="codex" />
              </div>
              <div className="min-w-0 md:col-span-1">
                <CostSummaryCard summary={codexSummary} />
              </div>
              <div className="min-w-0 md:col-span-3">
                <HeatmapSection source="codex" />
              </div>
              <div className="min-w-0 md:col-span-2 md:row-span-2">
                <CostCard summary={codexSummary} />
              </div>
              <div className="min-w-0 md:col-span-1 md:row-span-2">
                <ModelShareBar summary={codexSummary} />
              </div>
            </div>
          ) : codexStatusQuery.data ? (
            <CodexMissingRow side="Windows 侧" />
          ) : null}
        </div>
      )}
    </div>
  );
}
