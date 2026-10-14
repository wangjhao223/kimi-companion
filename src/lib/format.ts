/** token 数量的显示单位。 */
export type Unit = "k" | "M" | "B";

/** 按选定单位格式化 token 数：unit=B → "0.229B"，unit=M → "22.59M"，unit=k → "22588.5k"。 */
export function formatTokens(n: number, unit: Unit): string {
  if (unit === "B") return trim(n / 1_000_000_000, 3) + "B";
  if (unit === "M") return trim(n / 1_000_000, 2) + "M";
  return trim(n / 1_000, 1) + "k";
}

function trim(v: number, digits: number): string {
  return v.toFixed(digits).replace(/\.?0+$/, "");
}

/** 缩写时要剥掉的供应商前缀。 */
const VENDOR_PREFIXES = new Set(["kimi", "claude", "deepseek", "qwen", "glm", "gemini", "moonshot", "openai"]);

/** 模型名缩写：去掉供应商前缀、日期段（4/8 位纯数字）和 preview/latest 等修饰词，例 "kimi-k2-0905-preview" → "k2"、"gpt-5" → "gpt-5"。 */
export function abbrevModel(name: string): string {
  const stop = new Set(["preview", "latest", "exp", "beta", "alpha", "instruct"]);
  const parts = name.split("-").filter((p) => {
    if (/^\d{4}$/.test(p) || /^\d{8}$/.test(p)) return false;
    return !stop.has(p.toLowerCase());
  });
  if (parts.length > 1 && VENDOR_PREFIXES.has(parts[0].toLowerCase())) parts.shift();
  return parts.length > 0 ? parts.join("-") : name;
}
