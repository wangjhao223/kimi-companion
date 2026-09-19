//! DesktopService：Windows 桌面程序（Kimi Code）的安装检测、运行检测与启动。
//!
//! - 安装检测：先查候选 exe 路径存在性，再用 `reg query` 枚举注册表卸载项，
//!   按 DisplayName 匹配后解析 DisplayIcon（形如 `C:\Kimi Code\Kimi Code.exe,0`，
//!   需去掉引号与 `,<n>` 图标索引后缀）兜底；
//! - 运行检测：`tasklist /NH /FO CSV /FI "IMAGENAME eq <进程名>"`，
//!   CSV 输出任一行的映像名与进程名一致即运行中（无匹配时 tasklist 只输出
//!   一行本地化 INFO 文本，天然不会误判）；
//! - 启动：std::process::Command 直接 spawn exe（Windows GUI 程序，spawn 后即独立运行）；
//! - 只用 std + 已有依赖；所有外部命令失败一律降级为未安装/未运行，不 panic。

use std::path::PathBuf;
use std::process::Command;

use crate::models::DesktopAppInfo;

/// 拉起控制台子进程（reg / tasklist）。GUI 应用里必须加 CREATE_NO_WINDOW，
/// 否则每轮状态轮询都会闪出控制台黑框（与 wsl.rs 的 wsl_command 同一处理）。
fn console_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 一个受支持的桌面程序的静态描述。
struct AppSpec {
    /// 固定标识，前端按此传参
    id: &'static str,
    /// 展示名
    name: &'static str,
    /// 候选 exe 绝对路径（优先按存在性检测）
    exe_candidates: &'static [&'static str],
    /// tasklist 过滤用的进程映像名
    process_name: &'static str,
    /// 注册表卸载项根（全写 hive 名，与 reg query 输出回显一致）
    uninstall_roots: &'static [&'static str],
    /// DisplayName 匹配规则（版本号会变，Kimi Code 按前缀匹配）
    display_name_match: fn(&str) -> bool,
}

const APPS: &[AppSpec] = &[AppSpec {
    id: "kimi-code",
    name: "Kimi Code",
    exe_candidates: &[r"C:\Program Files\Kimi Code\Kimi Code.exe"],
    process_name: "Kimi Code.exe",
    uninstall_roots: &[
        r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ],
    // DisplayName 形如 "Kimi Code 1.0.1"，版本号随升级变化
    display_name_match: |n| n.starts_with("Kimi Code"),
}];

/// 全部桌面应用的安装/运行状态快照。
pub fn desktop_apps_status() -> Vec<DesktopAppInfo> {
    APPS.iter()
        .map(|spec| {
            let exe_path = find_exe(spec);
            DesktopAppInfo {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                installed: exe_path.is_some(),
                running: is_running(spec.process_name),
                exe_path: exe_path.map(|p| p.to_string_lossy().into_owned()),
            }
        })
        .collect()
}

/// 启动指定桌面应用。app 为 APPS 中的 id（当前仅 "kimi-code"）；未安装时报中文错误。
pub fn start_app(app: &str) -> Result<(), String> {
    let Some(spec) = APPS.iter().find(|s| s.id == app) else {
        return Err(format!("未知的桌面应用: {app}"));
    };
    let Some(exe) = find_exe(spec) else {
        return Err(format!("{} 未安装，无法启动", spec.name));
    };
    Command::new(&exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("启动 {} 失败: {e}", spec.name))
}

/// 安装检测：候选路径存在性优先，注册表卸载项 DisplayIcon 兜底。
fn find_exe(spec: &AppSpec) -> Option<PathBuf> {
    for candidate in spec.exe_candidates {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Some(p);
        }
    }
    find_exe_via_registry(spec)
}

/// 注册表兜底：枚举卸载项子键，DisplayName 匹配后解析 DisplayIcon 得到 exe 路径。
fn find_exe_via_registry(spec: &AppSpec) -> Option<PathBuf> {
    for root in spec.uninstall_roots {
        for key in enum_uninstall_keys(root) {
            let Some(display_name) = query_reg_value(&key, "DisplayName") else {
                continue;
            };
            if !(spec.display_name_match)(&display_name) {
                continue;
            }
            let Some(icon) = query_reg_value(&key, "DisplayIcon") else {
                continue;
            };
            let Some(exe) = sanitize_display_icon(&icon) else {
                continue;
            };
            let p = PathBuf::from(exe);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// 列出卸载项根下的全部子键路径。reg 输出每个子键独占一行、以根路径开头。
fn enum_uninstall_keys(root: &str) -> Vec<String> {
    let Ok(output) = console_command("reg").args(["query", root]).output() else {
        return Vec::new();
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let prefix = format!("{root}\\");
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| l.len() > prefix.len() && l.starts_with(&prefix))
        .map(String::from)
        .collect()
}

/// 读取某键的指定 REG_SZ 值。输出行形如 `    DisplayIcon    REG_SZ    C:\Kimi Code\Kimi Code.exe,0`，
/// 以 REG_SZ 为锚点切出值名与数据，值名不区分大小写。
fn query_reg_value(key: &str, value: &str) -> Option<String> {
    let output = console_command("reg")
        .args(["query", key, "/v", value])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        let Some(pos) = line.find("REG_SZ") else {
            continue;
        };
        if !line[..pos].trim().eq_ignore_ascii_case(value) {
            continue;
        }
        return Some(line[pos + "REG_SZ".len()..].trim().to_string());
    }
    None
}

/// DisplayIcon 值 → exe 路径：去引号、去末尾 `,<n>` 图标索引后缀；非 .exe 一律放弃。
fn sanitize_display_icon(raw: &str) -> Option<String> {
    let mut s = raw.trim().trim_matches('"').to_string();
    if let Some(pos) = s.rfind(',') {
        let idx = &s[pos + 1..];
        if !idx.is_empty() && idx.bytes().all(|b| b.is_ascii_digit()) {
            s.truncate(pos);
        }
    }
    let s = s.trim().to_string();
    if s.to_ascii_lowercase().ends_with(".exe") {
        Some(s)
    } else {
        None
    }
}

/// 运行检测：tasklist CSV 输出，任一行首列（映像名）与进程名一致（不区分大小写）。
fn is_running(process_name: &str) -> bool {
    let Ok(output) = console_command("tasklist")
        .args([
            "/NH",
            "/FO",
            "CSV",
            "/FI",
            &format!("IMAGENAME eq {process_name}"),
        ])
        .output()
    else {
        return false;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().any(|line| {
        line.trim()
            .split(',')
            .next()
            .map(|col| col.trim_matches('"'))
            .is_some_and(|img| img.eq_ignore_ascii_case(process_name))
    })
}
