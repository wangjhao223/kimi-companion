/** token 数量的显示单位。 */
export type Unit = "k" | "M";

/** 按选定单位格式化 token 数：unit=M → "22.59M"，unit=k → "22588.5k"。 */
export function formatTokens(n: number, unit: Unit): string {
  if (unit === "M") return trim(n / 1_000_000, 2) + "M";
  return trim(n / 1_000, 1) + "k";
}

function trim(v: number, digits: number): string {
  return v.toFixed(digits).replace(/\.?0+$/, "");
}
