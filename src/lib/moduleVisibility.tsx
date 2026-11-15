import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type ModuleId =
  | "control"
  | "summary"
  | "rates"
  | "costSummary"
  | "heatmap"
  | "speed"
  | "trend"
  | "costDetail"
  | "modelShare"
  | "update"
  | "ledger";

interface ModuleDefinition {
  id: ModuleId;
  label: string;
  /** 设置弹窗中的分组标题，相同分组的模块连续排列。 */
  group?: string;
  /** 设置弹窗中的补充说明。 */
  description?: string;
}

/** 可显示/隐藏的界面模块，顺序即设置弹窗中的默认排列顺序。 */
export const MODULE_DEFINITIONS: readonly ModuleDefinition[] = [
  {
    id: "control",
    label: "总控/状态",
    group: "总览",
    description: "启动、停止与运行状态",
  },
  {
    id: "summary",
    label: "Token 计数",
    group: "总览",
    description: "Token 总量与本期汇总",
  },
  {
    id: "rates",
    label: "近 60 分钟速率",
    group: "用量",
    description: "最近一小时的 RPM / TPM 曲线",
  },
  {
    id: "heatmap",
    label: "年度热力图",
    group: "用量",
    description: "按天统计的年度用量分布",
  },
  {
    id: "speed",
    label: "输出速度",
    group: "用量",
    description: "最近五分钟的输出速度",
  },
  {
    id: "trend",
    label: "用量趋势",
    group: "用量",
    description: "按时间范围统计的用量趋势",
  },
  {
    id: "modelShare",
    label: "模型占比",
    group: "用量",
    description: "各模型的 Token 占比",
  },
  {
    id: "costSummary",
    label: "费用估算",
    group: "费用",
    description: "当前用量的费用快速估算",
  },
  {
    id: "costDetail",
    label: "费用明细（单价）",
    group: "费用",
    description: "逐事件条件计费与模型单价明细",
  },
  {
    id: "update",
    label: "Kimi Code 更新",
    group: "更新与同步",
    description: "检查并安装 Kimi Code 更新",
  },
  {
    id: "ledger",
    label: "账本同步",
    group: "更新与同步",
    description: "账本记录数与最近同步时间",
  },
];

const STORAGE_KEY = "kimi-companion.module-visibility";

const DEFAULT_VISIBILITY: Record<ModuleId, boolean> = {
  control: true,
  summary: true,
  rates: true,
  costSummary: true,
  heatmap: true,
  speed: true,
  trend: true,
  costDetail: true,
  modelShare: true,
  update: true,
  ledger: true,
};

interface ModuleVisibilityValue {
  isVisible: (id: ModuleId) => boolean;
  setVisible: (id: ModuleId, visible: boolean) => void;
  setAll: (visible: boolean) => void;
}

/** 读取本地配置：损坏、缺字段或无法访问时按“全部显示”处理，旧配置缺失的新模块也默认显示。 */
function readStoredVisibility(): Record<ModuleId, boolean> {
  const visibility: Record<ModuleId, boolean> = { ...DEFAULT_VISIBILITY };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return visibility;
    const parsed: unknown = JSON.parse(raw);
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
      return visibility;
    }
    for (const { id } of MODULE_DEFINITIONS) {
      const value = (parsed as Record<string, unknown>)[id];
      if (typeof value === "boolean") visibility[id] = value;
    }
  } catch {
    // JSON 损坏或 localStorage 不可用时回到默认值
  }
  return visibility;
}

const ModuleVisibilityContext = createContext<ModuleVisibilityValue>({
  isVisible: () => true,
  setVisible: () => {},
  setAll: () => {},
});

/** 全局模块显示/隐藏状态，持久化到 localStorage；写入失败时仅保留当前会话状态。 */
export function ModuleVisibilityProvider({ children }: { children: ReactNode }) {
  const [visibility, setVisibility] =
    useState<Record<ModuleId, boolean>>(readStoredVisibility);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(visibility));
    } catch {
      // localStorage 不可用时仅保留内存态
    }
  }, [visibility]);

  const isVisible = useCallback(
    (id: ModuleId) => visibility[id] !== false,
    [visibility]
  );

  const setVisible = useCallback((id: ModuleId, visible: boolean) => {
    setVisibility((prev) => ({ ...prev, [id]: visible }));
  }, []);

  const setAll = useCallback((visible: boolean) => {
    setVisibility(() => {
      const next = {} as Record<ModuleId, boolean>;
      for (const { id } of MODULE_DEFINITIONS) next[id] = visible;
      return next;
    });
  }, []);

  const value = useMemo(
    () => ({ isVisible, setVisible, setAll }),
    [isVisible, setVisible, setAll]
  );

  return (
    <ModuleVisibilityContext.Provider value={value}>
      {children}
    </ModuleVisibilityContext.Provider>
  );
}

export function useModuleVisibility() {
  return useContext(ModuleVisibilityContext);
}