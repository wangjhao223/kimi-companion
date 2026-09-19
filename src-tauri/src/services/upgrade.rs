//! UpgradeService：kimi CLI 的版本查询 / 更新检查 / 执行更新。
//!
//! 实测行为（2.0.1）：
//! - `kimi --version` 输出纯版本号（如 "2.0.1"）；
//! - `kimi upgrade`（不带 -y）已最新时输出 "... already up to date ..." 并以 0 退出；
//!   有新版本时打印版本信息并弹确认提示——stdin 为 null（见 wsl::run_in_wsl_full），
//!   提示符拿到 EOF 立即中止，不会卡住；
//! - `kimi upgrade -y` 跳过确认直接安装（下载约 178MB，WSL 侧 timeout 600 兜底）。
//!
//! 超时用 WSL 侧 `timeout` 命令实现，其杀掉被包进程时退出码 124，单独识别
//! 并映射为中文超时错误。

use crate::commands::launcher::SharedInstance;
use crate::models::{UpdateCheckResult, UpdateResult};
use crate::services::{launcher, wsl};

/// WSL 侧 `timeout` 命令杀掉被包进程时的退出码
const TIMEOUT_EXIT_CODE: i32 = 124;

/// CLI 原始输出取 stdout，为空时退回 stderr。
fn raw_output(out: &wsl::WslOutput) -> String {
    if out.stdout.is_empty() {
        out.stderr.clone()
    } else {
        out.stdout.clone()
    }
}

/// `kimi --version`：返回纯版本号。
///
/// bash -ls 加载的 profile 可能往 stdout 打杂讯（与 wsl::probe 同坑），
/// 取最后一个非空行。
pub fn get_version(distro: &str) -> Result<String, String> {
    let out = wsl::run_in_wsl(distro, "kimi --version")
        .map_err(|e| format!("获取 kimi CLI 版本失败: {e}"))?;
    let version = out
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string();
    if version.is_empty() {
        return Err(format!(
            "kimi --version 输出为空（发行版 {distro} 里是否已安装 kimi CLI？）"
        ));
    }
    Ok(version)
}

/// 检查更新：`timeout 90 kimi upgrade`（绝不加 -y，避免误触发安装）。
///
/// 非 0 退出码不视为失败：有新版本时确认提示被 EOF 中止，退出码未必为 0，
/// 输出照样解析；解析不出就 up_to_date=false、latest=None，原样返回 output。
pub fn check_update(distro: &str) -> Result<UpdateCheckResult, String> {
    // 先拿当前版本：kimi 未安装时在这里就报出明确错误
    let current_version = get_version(distro)?;

    let out = wsl::run_in_wsl_full(distro, "timeout 90 kimi upgrade")?;
    if out.code == Some(TIMEOUT_EXIT_CODE) {
        return Err("检查更新超时（90 秒），请检查网络连接后重试".to_string());
    }

    let output = raw_output(&out);
    let up_to_date = output.contains("already up to date");
    // 输出里的版本号可能有多个（当前版本也会出现在输出中），
    // 取第一个不等于当前版本的作为最新版本
    let latest_version = if up_to_date {
        None
    } else {
        extract_versions(&output)
            .into_iter()
            .find(|v| v != &current_version)
    };

    Ok(UpdateCheckResult {
        current_version,
        up_to_date,
        latest_version,
        output,
    })
}

/// 执行更新：`timeout 600 kimi upgrade -y`（178MB 下载给足时间）。
/// 成功后重读版本号，且若 kimi web 在跑则自动重启它。
pub fn update(state: &SharedInstance, distro: &str) -> Result<UpdateResult, String> {
    let out = wsl::run_in_wsl_full(distro, "timeout 600 kimi upgrade -y")?;
    if out.code == Some(TIMEOUT_EXIT_CODE) {
        return Err("更新超时（600 秒），请检查网络连接后重试".to_string());
    }

    let output = raw_output(&out);
    let success = out.code == Some(0);
    let (new_version, restarted_web) = if success {
        // 版本读不到 / web 重启失败都不推翻更新本身的成功
        let new_version = get_version(distro).ok();
        let restarted_web = launcher::restart_web_if_running(state, distro).unwrap_or(false);
        (new_version, restarted_web)
    } else {
        (None, false)
    };

    Ok(UpdateResult {
        success,
        output,
        restarted_web,
        new_version,
    })
}

/// 提取文本中所有形如 `v?X.Y.Z` 的版本号（返回不含 v 前缀的 "X.Y.Z"）。
/// 等价于正则 v?(\d+\.\d+\.\d+) 的从左到右全量匹配（无 regex 依赖，手写解析）。
fn extract_versions(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut versions = Vec::new();
    let mut i = 0;
    while i < b.len() {
        // 起点：可选的 'v'（后须跟数字）或直接是数字
        let start = if b[i] == b'v' && b.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            i + 1
        } else if b[i].is_ascii_digit() {
            i
        } else {
            i += 1;
            continue;
        };
        match parse_semver_end(b, start) {
            Some(end) => {
                // start/end 均落在 ASCII 字符边界上，切片安全
                versions.push(text[start..end].to_string());
                i = end;
            }
            None => i += 1,
        }
    }
    versions
}

/// 从 start 解析 `数字+.数字+.数字+`，成功返回结束下标（不含）。
fn parse_semver_end(b: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    for seg in 0..3 {
        if seg > 0 {
            if b.get(i) != Some(&b'.') {
                return None;
            }
            i += 1;
        }
        let seg_start = i;
        while b.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1;
        }
        if i == seg_start {
            return None;
        }
    }
    Some(i)
}
