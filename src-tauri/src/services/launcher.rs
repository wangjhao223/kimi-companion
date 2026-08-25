//! LauncherService：kimi web + kimi TUI 的启动 / 状态探测 / 停止。
//!
//! 启动流程（见方案四.2）：
//! 1. 端口区间健康探测，已有健康实例则复用（不用 pgrep，见 `start` 注释）；
//! 2. 否则由长驻 wsl.exe 承载 `kimi web` 前台进程拉起（见 wsl::spawn_persistent）；
//! 3. 轮询 `http://127.0.0.1:<port>/api/v1/healthz`（端口 58627~58637 逐个试，最多 30 秒，
//!    依赖 WSL2 的 localhost 转发）；
//! 4. 健康后读 `~/.kimi-code/server.token` 并调认证接口验证，写入 AppState；
//! 5. 没有 kimi TUI 在跑时，弹可见终端窗口启动它。
//!
//! 停止流程：shutdown API 优雅停 web → `pkill -x kimi` 清剿全部 kimi 进程（含 TUI）。

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

use crate::models::{LaunchStatus, RunningInstance};
use crate::services::http::shared_client;
use crate::services::wsl::{self, WslProbe};

/// kimi web 自己会 port+1 重试，因此探测一段端口区间。
pub const PORT_RANGE: std::ops::RangeInclusive<u16> = 58627..=58637;
const HEALTH_WAIT_MAX: Duration = Duration::from_secs(30);
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(500);

// WSL 探测缓存：合并后的单次探测（wsl::probe）仍有 wsl.exe 进程拉起成本，
// 状态轮询每 2.5s 一次全量跑不划算，按 TTL 缓存；start/stop 后主动失效。
struct ProbeCacheEntry {
    at: Instant,
    distro: String,
    probe: WslProbe,
}

const PROBE_CACHE_TTL: Duration = Duration::from_secs(10);

fn probe_cache() -> &'static Mutex<Option<ProbeCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<ProbeCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn cached_probe(distro: &str) -> Result<WslProbe, String> {
    if let Ok(guard) = probe_cache().lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.distro == distro && entry.at.elapsed() < PROBE_CACHE_TTL {
                return Ok(entry.probe.clone());
            }
        }
    }
    let probe = wsl::probe(distro)?;
    if let Ok(mut guard) = probe_cache().lock() {
        *guard = Some(ProbeCacheEntry {
            at: Instant::now(),
            distro: distro.to_string(),
            probe: probe.clone(),
        });
    }
    Ok(probe)
}

/// start/stop 改变了 WSL 侧状态，下一轮轮询必须拿到新结果。
fn invalidate_probe_cache() {
    if let Ok(mut guard) = probe_cache().lock() {
        *guard = None;
    }
}

fn healthz(client: &Client, port: u16, timeout: Duration) -> bool {
    client
        .get(format!("http://127.0.0.1:{port}/api/v1/healthz"))
        .timeout(timeout)
        .send()
        .map(|resp| resp.status().is_success())
        .unwrap_or(false)
}

/// 扫描端口区间，返回第一个健康检查通过的端口。
fn probe_running(client: &Client, timeout: Duration) -> Option<u16> {
    PORT_RANGE
        .into_iter()
        .find(|&port| healthz(client, port, timeout))
}

fn build_url(port: u16, token: &str) -> String {
    format!("http://127.0.0.1:{port}/#token={token}")
}

/// 查询当前状态。distro 为 None 时跳过 WSL 侧检查（只做端口探测）。
pub fn get_status(state: &Arc<Mutex<Option<RunningInstance>>>, distro: Option<&str>) -> LaunchStatus {
    // 探测请求走 loopback，未监听时 connect 会立刻失败，500ms 超时足够。
    let client = match shared_client() {
        Ok(c) => c,
        Err(_) => {
            return LaunchStatus {
                wsl_ok: false,
                kimi_installed: false,
                running: false,
                tui_running: false,
                port: None,
                url: None,
            }
        }
    };
    let port = probe_running(client, Duration::from_millis(500));

    let (wsl_ok, kimi_installed, tui_running) = match distro {
        Some(d) => match cached_probe(d) {
            Ok(p) => (true, p.kimi_installed, p.tui_running),
            Err(_) => (false, false, false),
        },
        None => (false, false, false),
    };

    // 接管非本应用启动的实例：发现健康端口但状态里没有（或端口不一致）时，
    // 从 WSL 读 token 纳入管理——否则「停止」和完整 URL 只对亲手启动的实例可用。
    if let (Some(port), Some(d)) = (port, distro) {
        let known = state
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|inst| inst.port == port))
            .unwrap_or(false);
        if !known {
            if let Ok(token) = wsl::run_in_wsl(d, "cat ~/.kimi-code/server.token") {
                if !token.is_empty() {
                    if let Ok(mut guard) = state.lock() {
                        *guard = Some(RunningInstance {
                            distro: d.to_string(),
                            port,
                            token,
                        });
                    }
                }
            }
        }
    }

    // token 只在实例由本应用启动/接管过的情况下可用
    let url = state
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .filter(|inst| Some(inst.port) == port)
        .map(|inst| build_url(inst.port, &inst.token));

    LaunchStatus {
        wsl_ok,
        kimi_installed,
        running: port.is_some(),
        tui_running,
        port,
        url,
    }
}

/// 一键启动：已有健康实例则复用，否则拉起新进程并等待健康。
///
/// 注意：不能用 `pgrep -f 'kimi web'` 判断——模式串会匹配到 pgrep 自己的
/// 外层 shell 命令行（自我匹配），且 kimi 进程会改写 argv 导致 'kimi web'
/// 根本匹配不到真实进程。以端口健康检查作为「是否在跑」的唯一判据。
pub fn start(state: &Arc<Mutex<Option<RunningInstance>>>, distro: &str) -> Result<LaunchStatus, String> {
    // 1. 已在跑则复用
    let client = shared_client()?;
    let already_running = probe_running(client, Duration::from_secs(2)).is_some();

    // 未在跑时才需要探测安装情况（一次调用同时拿到 TUI 状态，供第 4 步用）
    let probe0 = if already_running {
        None
    } else {
        Some(wsl::probe(distro)?)
    };
    if let Some(p) = &probe0 {
        // 校验 kimi 已安装，否则等 30 秒也只会超时
        if !p.kimi_installed {
            return Err(format!("发行版 {distro} 中没有找到 kimi CLI（which kimi 为空）"));
        }
        // 由长驻 wsl.exe 承载 kimi web 前台进程（见 spawn_persistent 注释），
        // 日志追加写入，便于排查多次重启。先 cd ~：wsl.exe 会把 Windows 侧的
        // 工作目录翻译成 Linux 起始目录，不固定的话会落在 /mnt/c/Users/<用户>。
        wsl::spawn_persistent(
            distro,
            "cd ~ && mkdir -p ~/.kimi-code && exec kimi web >> ~/.kimi-code/companion-web.log 2>&1",
        )?;
    }

    // 2. 轮询健康检查直到就绪，3. 读 token 并验证
    let (port, token) = wait_ready_and_verify(client, distro)?;

    // 4. 没有 kimi TUI 在跑时，弹一个可见终端窗口启动它（失败不阻塞主流程）
    let tui_running = match &probe0 {
        Some(p) => p.tui_running,
        None => wsl::probe(distro).map(|p| p.tui_running).unwrap_or(false),
    };
    if !tui_running {
        let _ = wsl::spawn_visible_terminal(distro, "cd ~ && exec kimi");
    }
    // WSL 侧状态已改变，强制下一轮状态轮询重新探测
    invalidate_probe_cache();

    let instance = RunningInstance {
        distro: distro.to_string(),
        port,
        token: token.clone(),
    };
    let mut guard = state.lock().map_err(|_| "应用状态锁已损坏".to_string())?;
    *guard = Some(instance);
    drop(guard);

    Ok(LaunchStatus {
        wsl_ok: true,
        kimi_installed: true,
        running: true,
        tui_running: true, // 刚拉起或已在跑
        port: Some(port),
        url: Some(build_url(port, &token)),
    })
}

/// 轮询健康检查直到拿到就绪端口（最多 30 秒），再读 token 并验证。
fn wait_ready_and_verify(client: &Client, distro: &str) -> Result<(u16, String), String> {
    // 健康检查用稍长的超时，容忍 WSL 转发慢
    let deadline = Instant::now();
    let port = loop {
        if let Some(port) = probe_running(client, Duration::from_secs(2)) {
            break port;
        }
        if deadline.elapsed() >= HEALTH_WAIT_MAX {
            return Err(format!(
                "等待 kimi web 就绪超时（{} 秒）。请检查 WSL 内 ~/.kimi-code/companion-web.log",
                HEALTH_WAIT_MAX.as_secs()
            ));
        }
        std::thread::sleep(HEALTH_POLL_INTERVAL);
    };
    // 读 token 并用认证接口验证（文件里的 token 可能与运行中的实例不匹配）
    let token = read_verified_token(client, distro, port)?;
    Ok((port, token))
}

/// 安装/更新记账 hook 后调用：kimi web 只在启动时加载一次 hooks 配置，
/// 已在运行的实例不会加载新配置（实测确认），因此 web 在跑就重启它让记账
/// 立即生效。只结束 web 服务进程（pid 取自实例文件），TUI 会话不受影响。
/// 返回是否执行了重启。
pub fn restart_web_if_running(
    state: &Arc<Mutex<Option<RunningInstance>>>,
    distro: &str,
) -> Result<bool, String> {
    let client = shared_client()?;
    if probe_running(client, Duration::from_millis(800)).is_none() {
        return Ok(false);
    }

    // 结束 web 服务进程。shutdown API 在 token 不匹配时会失败，直接按实例
    // 文件里的 pid kill 最稳；|| true 兜底保证命令恒以 0 退出。
    wsl::run_in_wsl(
        distro,
        "kill $(grep -o '\"pid\":[0-9]*' ~/.kimi-code/server/instances/*.json 2>/dev/null | cut -d: -f2) 2>/dev/null || true",
    )?;

    // 等端口清空（最多 10 秒）
    let deadline = Instant::now();
    while probe_running(client, Duration::from_millis(500)).is_some() {
        if deadline.elapsed() >= Duration::from_secs(10) {
            return Err("kimi web 进程未在 10 秒内退出".to_string());
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    wsl::spawn_persistent(
        distro,
        "cd ~ && mkdir -p ~/.kimi-code && exec kimi web >> ~/.kimi-code/companion-web.log 2>&1",
    )?;
    let (port, token) = wait_ready_and_verify(client, distro)?;

    let mut guard = state.lock().map_err(|_| "应用状态锁已损坏".to_string())?;
    *guard = Some(RunningInstance {
        distro: distro.to_string(),
        port,
        token,
    });
    drop(guard);
    invalidate_probe_cache();
    Ok(true)
}

/// 读 server.token 并调认证接口验证；验证失败时重读一次再验（应对写入时序竞争）。
fn read_verified_token(client: &Client, distro: &str, port: u16) -> Result<String, String> {
    for attempt in 0..2 {
        let token = wsl::run_in_wsl(distro, "cat ~/.kimi-code/server.token")?;
        if token.is_empty() {
            return Err("~/.kimi-code/server.token 为空或不存在".to_string());
        }
        let ok = client
            .get(format!("http://127.0.0.1:{port}/api/v1/oauth/usage"))
            .bearer_auth(&token)
            .timeout(Duration::from_secs(2))
            .send()
            .map(|resp| resp.status().is_success())
            .unwrap_or(false);
        if ok {
            return Ok(token);
        }
        if attempt == 0 {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    Err("server.token 与运行中的 kimi web 不匹配（认证失败）。请在 WSL 里重启 kimi web 或执行 kimi web rotate-token 后重试".to_string())
}

/// 停止：先调 shutdown API 优雅停掉 kimi web，再清剿该发行版内所有 kimi
/// 进程（web 若未退出则补杀 + 全部 TUI 会话）。
pub fn stop(state: &Arc<Mutex<Option<RunningInstance>>>) -> Result<(), String> {
    let instance = {
        let guard = state.lock().map_err(|_| "应用状态锁已损坏".to_string())?;
        guard.clone()
    };

    // 有实例记录就先走优雅关停；没有（例如状态丢失）也要继续清剿 kimi 进程
    if let Some(inst) = &instance {
        if let Ok(client) = shared_client() {
            let _ = client
                .post(format!("http://127.0.0.1:{}/api/v1/shutdown", inst.port))
                .bearer_auth(&inst.token)
                .timeout(Duration::from_secs(5))
                .send();
        }
        // 给 web 服务一点优雅退出时间
        std::thread::sleep(Duration::from_millis(800));
    }

    let distro = instance
        .map(|inst| inst.distro)
        .or_else(|| wsl::list_distros().ok()?.into_iter().next())
        .ok_or_else(|| "没有可用的 WSL 发行版".to_string())?;

    wsl::kill_all_kimi(&distro)?;
    invalidate_probe_cache();

    let mut guard = state.lock().map_err(|_| "应用状态锁已损坏".to_string())?;
    *guard = None;
    Ok(())
}
