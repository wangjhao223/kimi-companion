//! ModelCatalogService：读取 Kimi Code 的模型清单，只向前端返回模型名。
//!
//! - source=kimi：在指定 WSL 发行版内执行 `kimi provider list --json`（经 wsl::run_in_wsl）；
//! - source=kimi-win：直接读取 `%USERPROFILE%\.kimi-code\config.toml`。Windows PATH 里的
//!   `kimi` 可能是已停维护的旧 kimi-cli，不能再依赖它查询 Kimi Code 配置；
//! - 解析根对象 `models` 的每一项，按 `${provider}/${model}` 生成名称，provider 开头的
//!   `managed:` 去掉（与账本里的 `kimi-code/k3` 对齐），去重排序；
//! - 只返回模型名，原始配置（providers 里含 API key）绝不透传到前端；
//! - Codex 没有 provider 清单，模型筛选沿用历史用量，因此直接报错。

use std::collections::BTreeSet;

use serde_json::Value;

use super::wsl;

/// 查询指定数据源的可用模型名。
///
/// `source` 仅支持 `kimi` / `kimi-win`；`kimi` 必须提供非空的 `distro`。
pub fn get_available_models(source: &str, distro: Option<&str>) -> Result<Vec<String>, String> {
    match source {
        "kimi" => {
            let distro = distro
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .ok_or_else(|| "查询 WSL 模型清单需要指定发行版".to_string())?;
            let output = wsl::run_in_wsl(distro, "kimi provider list --json")
                .map_err(|e| format!("读取 kimi 模型清单失败: {e}"))?;
            parse_models(&output)
        }
        "kimi-win" => parse_toml_models(&read_windows_config()?),
        "codex" | "wsl-codex" => {
            Err("Codex 没有 provider 清单，模型筛选应沿用历史用量".to_string())
        }
        other => Err(format!("不支持的模型来源: {other}")),
    }
}

/// Windows 桌面端没有可供 GUI 调用的独立 Kimi Code CLI，直接读活动配置文件。
fn read_windows_config() -> Result<String, String> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "无法定位 Windows 用户目录".to_string())?;
    let path = std::path::PathBuf::from(home)
        .join(".kimi-code")
        .join("config.toml");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("无法读取 Kimi Code 配置 {}: {e}", path.display()))
}

/// 解析 `kimi provider list --json` 的 stdout。
///
/// `bash -l` 加载的 profile 可能往 stdout 打杂讯，按首个 `{` 到最后一个 `}` 截取 JSON。
/// 只提取 `models.*.provider` 与 `models.*.model`，其余字段（含 providers.apiKey）一律忽略。
pub fn parse_models(output: &str) -> Result<Vec<String>, String> {
    let start = output
        .find('{')
        .ok_or_else(|| "kimi 模型清单输出中没有 JSON 对象".to_string())?;
    let end = output
        .rfind('}')
        .ok_or_else(|| "kimi 模型清单输出中没有完整的 JSON 对象".to_string())?;
    if end < start {
        return Err("kimi 模型清单输出中没有完整的 JSON 对象".to_string());
    }

    let root: Value = serde_json::from_str(&output[start..=end])
        .map_err(|e| format!("kimi 模型清单 JSON 解析失败: {e}"))?;
    let models = root
        .get("models")
        .and_then(Value::as_object)
        .ok_or_else(|| "kimi 模型清单缺少 models 对象".to_string())?;

    let mut names = BTreeSet::new();
    for entry in models.values() {
        let provider = entry
            .get("provider")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let model = entry
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let provider = provider.strip_prefix("managed:").unwrap_or(provider);
        if provider.is_empty() || model.is_empty() {
            continue;
        }
        names.insert(format!("{provider}/{model}"));
    }
    Ok(names.into_iter().collect())
}

/// 解析 Windows 桌面端 config.toml；与 JSON 路径使用相同的模型名归一化规则。
fn parse_toml_models(content: &str) -> Result<Vec<String>, String> {
    let root: toml::Value = toml::from_str(content)
        .map_err(|e| format!("Kimi Code config.toml 解析失败: {e}"))?;
    let models = root
        .get("models")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "Kimi Code 配置缺少 models 表".to_string())?;

    let mut names = BTreeSet::new();
    for entry in models.values() {
        let provider = entry
            .get("provider")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let model = entry
            .get("model")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let provider = provider.strip_prefix("managed:").unwrap_or(provider);
        if provider.is_empty() || model.is_empty() {
            continue;
        }
        names.insert(format!("{provider}/{model}"));
    }
    Ok(names.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::{parse_models, parse_toml_models};

    #[test]
    fn parses_windows_toml_models() {
        let input = r#"
[providers."managed:kimi-code"]
apiKey = "sk-should-never-leak"
[models."kimi-code/k3"]
provider = "managed:kimi-code"
model = "k3"
[models."deepseek/deepseek-flash"]
provider = "deepseek"
model = "deepseek-flash"
"#;
        assert_eq!(
            parse_toml_models(input).unwrap(),
            vec!["deepseek/deepseek-flash", "kimi-code/k3"]
        );
        assert!(parse_toml_models("[providers]").is_err());
    }

    #[test]
    fn parses_models_and_strips_managed_prefix() {
        let output = r#"
profile loaded (noise)
{
  "providers": {
    "managed:kimi-code": { "apiKey": "sk-should-never-leak" }
  },
  "models": {
    "kimi-code/k3": { "provider": "managed:kimi-code", "model": "k3" },
    "deepseek/deepseek-v4-pro": { "provider": "deepseek", "model": "deepseek-v4-pro" },
    "plain/other": { "provider": "kimi-code", "model": "other" }
  }
}
done
"#;
        assert_eq!(
            parse_models(output).unwrap(),
            vec![
                "deepseek/deepseek-v4-pro",
                "kimi-code/k3",
                "kimi-code/other",
            ]
        );
    }

    #[test]
    fn dedupes_models_from_managed_and_plain_providers() {
        let output = r#"{"models":{
            "a": { "provider": "managed:kimi-code", "model": "k3" },
            "b": { "provider": "kimi-code", "model": "k3" },
            "c": { "provider": "kimi-code", "model": "k3" }
        }}"#;
        assert_eq!(parse_models(output).unwrap(), vec!["kimi-code/k3"]);
    }

    #[test]
    fn tolerates_profile_noise_around_json() {
        let output = concat!(
            "welcome to bash\n",
            r#"{"models":{"x":{"provider":"deepseek","model":"x"}}}"#,
            "\n[profile] done"
        );
        assert_eq!(parse_models(output).unwrap(), vec!["deepseek/x"]);
    }

    #[test]
    fn skips_entries_without_provider_or_model() {
        let output = r#"{"models":{
            "ok": { "provider": "deepseek", "model": "x" },
            "missing-model": { "provider": "deepseek" },
            "missing-provider": { "model": "y" },
            "empty": { "provider": "", "model": "" },
            "empty-managed": { "provider": "managed:", "model": "z" }
        }}"#;
        assert_eq!(parse_models(output).unwrap(), vec!["deepseek/x"]);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_models("no json here").is_err());
        assert!(parse_models("{not valid json}").is_err());
        assert!(parse_models(r#"{"providers":{}}"#).is_err());
    }
}