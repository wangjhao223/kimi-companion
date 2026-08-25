//! WslService：枚举 WSL 发行版、在指定发行版内执行命令、合并探测 kimi 状态。

use std::process::Command;

use base64::Engine;

/// Windows 下 GUI 应用拉起 wsl.exe（控制台子系统）时必须加 CREATE_NO_WINDOW，
/// 否则每次调用都会闪出一个控制台窗口（状态轮询会反复调它，闪个不停）。
fn wsl_command(args: &[&str]) -> Command {
    let mut cmd = Command::new("wsl.exe");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 枚举本机已安装的 WSL 发行版（`wsl.exe -l -q`）。
///
/// 注意：`wsl.exe` 的 stdout 是 UTF-16LE 编码，需先转码再按行解析。
pub fn list_distros() -> Result<Vec<String>, String> {
    let output = wsl_command(&["-l", "-q"])
        .output()
        .map_err(|e| format!("无法执行 wsl.exe（WSL 是否已安装？）: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "wsl.exe -l -q 退出码非 0: {:?}",
            output.status.code()
        ));
    }

    let text = decode_wsl_output(&output.stdout);
    Ok(text
        .lines()
        .map(|line| line.trim().trim_matches('\0'))
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

/// 在指定发行版里执行一条 bash 命令，返回 stdout（trim 后）。
///
/// 注意（实测踩坑）：wsl.exe 会把 `--` 之后的参数重新拼接成一条命令字符串，
/// 交给 /bin/bash -c 再解析一层——脚本里的 $var / $(...) 会被这层外层 shell
/// 提前展开成空值（例如探测脚本恒输出 "installed= tui="）。因此命令统一
/// base64 编码传输、WSL 内解码后交给登录 shell 执行，规避双层展开。
pub fn run_in_wsl(distro: &str, cmd: &str) -> Result<String, String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(cmd);
    let wrapped = format!("echo {b64} | base64 -d | bash -ls");
    let output = wsl_command(&["-d", distro, "--", "bash", "-lc", &wrapped])
        .output()
        .map_err(|e| format!("无法执行 wsl.exe: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "WSL 命令失败（distro={distro}, 退出码 {:?}）: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 一次 WSL 合并探测的结果（单条 bash 命令拿到，省进程拉起开销）。
#[derive(Debug, Clone)]
pub struct WslProbe {
    /// 发行版里是否安装了 kimi CLI
    pub kimi_installed: bool,
    /// 是否有 kimi TUI（交互会话）在跑：所有 kimi 进程减去 web 服务进程
    pub tui_running: bool,
}

/// 合并探测脚本：一条命令输出 "installed=yes|no tui=yes|no"。
///
/// web 服务进程 PID 取自 ~/.kimi-code/server/instances/*.json；
/// kimi 进程会改写 argv，进程名一律为 "kimi"，其余进程即 TUI 会话。
const PROBE_SCRIPT: &str = r#"i=no; which kimi >/dev/null 2>&1 && i=yes; \
w=" $(grep -o '"pid":[0-9]*' ~/.kimi-code/server/instances/*.json 2>/dev/null | cut -d: -f2 | tr '\n' ' ')"; \
t=no; for p in $(pgrep -x kimi); do case " $w " in *" $p "*) ;; *) t=yes; break;; esac; done; \
echo "installed=$i tui=$t""#;

/// 自检：kimi 是否安装 + 是否有 TUI 在跑，单次 wsl.exe 调用完成。
///
/// 只解析最后一行输出：bash -l 加载的 profile 可能往 stdout 打杂讯。
pub fn probe(distro: &str) -> Result<WslProbe, String> {
    let out = run_in_wsl(distro, PROBE_SCRIPT)?;
    let last = out.lines().last().unwrap_or("");
    Ok(WslProbe {
        kimi_installed: last.split_whitespace().any(|t| t == "installed=yes"),
        tui_running: last.split_whitespace().any(|t| t == "tui=yes"),
    })
}

/// 杀掉发行版内所有 kimi 进程（web 服务和全部 TUI 会话）。
pub fn kill_all_kimi(distro: &str) -> Result<(), String> {
    run_in_wsl(distro, "pkill -x kimi || true")?;
    Ok(())
}

/// 弹出一个可见终端窗口运行命令（用于启动 kimi TUI；不加 CREATE_NO_WINDOW，
/// Windows 会为控制台子进程分配可见控制台窗口）。
#[cfg(windows)]
pub fn spawn_visible_terminal(distro: &str, cmd: &str) -> Result<(), String> {
    Command::new("wsl.exe")
        .args(["-d", distro, "--", "bash", "-lc", cmd])
        .spawn()
        .map_err(|e| format!("无法打开终端窗口: {e}"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn spawn_visible_terminal(_distro: &str, _cmd: &str) -> Result<(), String> {
    Err("spawn_visible_terminal 仅在 Windows 上可用".to_string())
}

/// 拉起一个长驻 WSL 前台进程（无窗口）。wsl.exe 保持运行，WSL 就不会回收会话。
///
/// 背景：在 `wsl.exe -- bash -c "... &"` 一次性命令里，即使用 setsid+nohup+
/// 三重重定向，后台进程也会在 wsl.exe 退出时被 WSL 回收（实测确认）。因此
/// 长驻进程必须由持续存活的 wsl.exe 承载——它在 Windows 侧以隐藏窗口运行，
/// kimi web 退出后它随之退出，天然跟随生命周期。
#[cfg(windows)]
pub fn spawn_persistent(distro: &str, cmd: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new("wsl.exe")
        .args(["-d", distro, "--", "bash", "-lc", cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("无法启动 wsl.exe 长驻进程: {e}"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn spawn_persistent(_distro: &str, _cmd: &str) -> Result<(), String> {
    Err("spawn_persistent 仅在 Windows 上可用".to_string())
}

/// `wsl.exe` 输出为 UTF-16LE（可能带 BOM）；解码失败时回退 UTF-8。
fn decode_wsl_output(bytes: &[u8]) -> String {
    let body = if bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        bytes
    };
    if !body.is_empty() && body.len() % 2 == 0 {
        let units: Vec<u16> = body
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&units) {
            return s;
        }
    }
    String::from_utf8_lossy(bytes).to_string()
}
