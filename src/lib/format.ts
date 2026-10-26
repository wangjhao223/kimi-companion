/** token 数量的显示单位；auto 按数量级在各挡位间自动切换。 */
export type Unit = "auto" | "k" | "M" | "B";

/** 按选定单位格式化 token 数：unit=auto 按绝对值自动选挡位（>=1e9 → B、>=1e6 → M、其余 → k），
 *  unit=B → "0.229B"，unit=M → "22.59M"，unit=k → "22588.5k"。 */
export function formatTokens(n: number, unit: Unit): string {
  if (unit === "auto") {
    const abs = Math.abs(n);
    if (abs >= 1_000_000_000) return trim(n / 1_000_000_000, 3) + "B";
    if (abs >= 1_000_000) return trim(n / 1_000_000, 2) + "M";
    return trim(n / 1_000, 1) + "k";
  }
  if (unit === "B") return trim(n / 1_000_000_000, 3) + "B";
  if (unit === "M") return trim(n / 1_000_000, 2) + "M";
  return trim(n / 1_000, 1) + "k";
}

function trim(v: number, digits: number): string {
  return v.toFixed(digits).replace(/\.?0+$/, "");
}

/** 无命名空间时要剥掉的供应商前缀（有 "/" 时保留完整模型名，不再二次剥离）。 */
const VENDOR_PREFIXES = new Set([
  "anthropic",
  "claude",
  "deepseek",
  "gemini",
  "glm",
  "kimi",
  "moonshot",
  "moonshotai",
  "openai",
  "qwen",
]);

/** 模型名缩写：去掉供应商命名空间、日期段（4/8 位纯数字）和 preview/latest 等修饰词。
 *  有命名空间时只去掉最后一个 "/" 前的部分，保留后面的完整模型名，不再二次剥离连字符前缀：
 *  "deepseek/deepseek-flash" → "deepseek-flash"、"claude/claude-opus-5-5" → "claude-opus-5-5"；
 *  无命名空间时才移除已知的连字符供应商前缀："kimi-k2-0905-preview" → "k2"、"gpt-5" → "gpt-5"。
 *  若没有剩余的具体模型部分则回退到原名，避免显示为空。 */
export function abbrevModel(name: string): string {
  const raw = name.trim();
  const hasNamespace = raw.includes("/");
  const base = hasNamespace ? raw.slice(raw.lastIndexOf("/") + 1).trim() || raw : raw;
  const stop = new Set(["preview", "latest", "exp", "beta", "alpha", "instruct"]);
  const parts = base.split("-").filter((p) => {
    if (!p) return false;
    if (/^\d{4}$/.test(p) || /^\d{8}$/.test(p)) return false;
    return !stop.has(p.toLowerCase());
  });
  if (!hasNamespace) {
    while (parts.length > 1 && VENDOR_PREFIXES.has(parts[0].toLowerCase())) parts.shift();
  }
  return parts.length > 0 ? parts.join("-") : raw;
}
