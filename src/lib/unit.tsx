import {
  createContext,
  useContext,
  useState,
  type ReactNode,
} from "react";
import type { Unit } from "./format";

const STORAGE_KEY = "kimi-companion.token-unit";

const UnitContext = createContext<{ unit: Unit; setUnit: (u: Unit) => void }>({
  unit: "M",
  setUnit: () => {},
});

/** 全局 token 显示单位（k / M），持久化到 localStorage。 */
export function UnitProvider({ children }: { children: ReactNode }) {
  const [unit, setUnitState] = useState<Unit>(() =>
    localStorage.getItem(STORAGE_KEY) === "k" ? "k" : "M"
  );
  const setUnit = (u: Unit) => {
    try {
      localStorage.setItem(STORAGE_KEY, u);
    } catch {
      // localStorage 不可用时仅保留内存态
    }
    setUnitState(u);
  };
  return (
    <UnitContext.Provider value={{ unit, setUnit }}>
      {children}
    </UnitContext.Provider>
  );
}

export function useUnit() {
  return useContext(UnitContext);
}
