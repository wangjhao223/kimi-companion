//! LauncherService：kimi web + kimi TUI 的启动 / 状态探测 / 停止。
//!
//! 启动流程（见方案四.2）：
//! 1. 端口区间健康探测，已有健康实例则复用（不用 pgrep，见 `start` 注释）；
//! 2. 否则由长驻 wsl.exe 承载 `kimi web` 前台进程拉起（见 wsl::spawn_persistent）；
//! 3. 轮询 `http://127.0.0.1:<port>/api/v1/healthz`（端口 58627~58637 逐个试，最多 30 秒，
//!    依赖 WSL2 的 localhost 转发）；
//! 4. 健康后读 `~/.kimi-code/server.token` 并调认证接口验证，写入 AppState。
//!    （不再自动弹终端启动 TUI——可见命令行窗口是噪音，web 不依赖它。）
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

/// 远程控制：健康后等待 RC 链接出现在日志里的最长时间 / 轮询间隔。
const RC_LINK_WAIT_MAX: Duration = Duration::from_secs(10);
const RC_LINK_POLL_INTERVAL: Duration = Duration::from_millis(500);
const RC_UNSUPPORTED_MSG: &str =
    "当前 kimi CLI 不支持远程控制（--rc），请先在「检查更新」模块升级 CLI";
const RC_LINK_TIMEOUT_MSG: &str =
    "远程控制链接生成超时，请检查网络（需能访问 code-rc.kimi.com）后重试";

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

/// 拉起 kimi web。rc=true 时带 `--rc`（远程控制），日志仍追加到同一文件。
fn spawn_web(distro: &str, rc: bool) -> Result<(), String> {
    let flag = if rc { " --rc" } else { "" };
    wsl::spawn_persistent(
        distro,
        &format!(
            "cd ~ && mkdir -p ~/.kimi-code && exec kimi web{flag} >> ~/.kimi-code/companion-web.log 2>&1"
        ),
    )
}

/// 记录日志当前行数。后续只解析新增行，避免把上一次启动留下的 RC 链接当成新链接。
/// 日志不存在时 wc 失败，按 0 处理。
fn log_line_offset(distro: &str) -> usize {
    wsl::run_in_wsl(
        distro,
        "wc -l < ~/.kimi-code/companion-web.log 2>/dev/null || echo 0",
    )
    .ok()
    .and_then(|out| {
        out.lines()
            .rev()
            .find_map(|line| line.trim().parse::<usize>().ok())
    })
    .unwrap_or(0)
}

enum RcLogProbe {
    /// 新日志里出现「未知参数」错误：旧版 CLI 不支持 --rc
    Unsupported,
    /// 解析到远程控制链接
    Url(String),
    /// 暂时还没有链接
    Pending,
}

/// 只扫描 offset 之后的新日志行：先剥 ANSI 颜色码（否则 ESC 会粘进 URL），
/// 再找 RC 链接；同时检查旧版 CLI 的 `unexpected argument` / `unrecognized`
/// 报错，避免等满健康检查/链接超时。
fn probe_rc_log(distro: &str, offset: usize) -> RcLogProbe {
    let from = offset.saturating_add(1);
    let script = format!(
        "L={from}; \
         if tail -n +$L ~/.kimi-code/companion-web.log 2>/dev/null | sed 's/\\x1b\\[[0-9;]*[A-Za-z]//g' | grep -qiE 'unexpected argument|unrecognized'; then \
           echo __RC_UNSUPPORTED__; \
         else \
           tail -n +$L ~/.kimi-code/companion-web.log 2>/dev/null | sed 's/\\x1b\\[[0-9;]*[A-Za-z]//g' | grep -oE 'https://code-rc\\.kimi\\.com/devices/[^[:space:]\"]+' | tail -1; \
         fi"
    );
    let out = match wsl::run_in_wsl(distro, &script) {
        Ok(out) => out,
        Err(_) => return RcLogProbe::Pending,
    };
    if out.lines().any(|line| line.trim() == "__RC_UNSUPPORTED__") {
        return RcLogProbe::Unsupported;
    }
    match out.lines().last().map(|line| line.trim()) {
        Some(url) if url.starts_with("https://code-rc.kimi.com/devices/") => {
            RcLogProbe::Url(url.to_string())
        }
        _ => RcLogProbe::Pending,
    }
}

enum RcLinkWait {
    Found(String),
    Unsupported,
    Timeout,
}

/// 健康后最多等 10 秒，每 500ms 扫一次新增日志找 RC 链接。
fn wait_for_rc_url(distro: &str, offset: usize) -> RcLinkWait {
    let started = Instant::now();
    loop {
        match probe_rc_log(distro, offset) {
            RcLogProbe::Url(url) => return RcLinkWait::Found(url),
            RcLogProbe::Unsupported => return RcLinkWait::Unsupported,
            RcLogProbe::Pending => {}
        }
        if started.elapsed() >= RC_LINK_WAIT_MAX {
            return RcLinkWait::Timeout;
        }
        std::thread::sleep(RC_LINK_POLL_INTERVAL);
    }
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
                port: None,
                url: None,
                remote_control: false,
                rc_url: None,
            }
        }
    };
    let port = probe_running(client, Duration::from_millis(500));

    let (wsl_ok, kimi_installed) = match distro {
        Some(d) => match cached_probe(d) {
            Ok(p) => (true, p.kimi_installed),
            Err(_) => (false, false),
        },
        None => (false, false),
    };

    // 接管非本应用启动的实例：发现健康端口但状态里没有（或端口不一致）时，
    // 从 WSL 读 token 纳入管理——否则「停止」和完整 URL 只对亲手启动的实例可用。
    // 外部启动的实例无法可靠探测是否开了 --rc，一律按未开启处理。
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
                            remote_control: false,
                            rc_url: None,
                        });
                    }
                }
            }
        }
    }

    // token / 远程控制标记只在实例由本应用启动/接管过、且端口仍匹配时可用
    let known = state
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .filter(|inst| Some(inst.port) == port);
    let (url, remote_control, rc_url) = match known {
        Some(inst) => (
            Some(build_url(inst.port, &inst.token)),
            inst.remote_control,
            inst.rc_url,
        ),
        None => (None, false, None),
    };

    LaunchStatus {
        wsl_ok,
        kimi_installed,
        running: port.is_some(),
        port,
        url,
        remote_control,
        rc_url,
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

    // 未在跑时才需要探测安装情况
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
        spawn_web(distro, false)?;
    }

    // 2. 轮询健康检查直到就绪，3. 读 token 并验证
    let (port, token) = wait_ready_and_verify(client, distro)?;

    // WSL 侧状态已改变，强制下一轮状态轮询重新探测
    invalidate_probe_cache();

    let instance = RunningInstance {
        distro: distro.to_string(),
        port,
        token: token.clone(),
        remote_control: false,
        rc_url: None,
    };
    let mut guard = state.lock().map_err(|_| "应用状态锁已损坏".to_string())?;
    *guard = Some(instance);
    drop(guard);

    Ok(LaunchStatus {
        wsl_ok: true,
        kimi_installed: true,
        running: true,
        port: Some(port),
        url: Some(build_url(port, &token)),
        remote_control: false,
        rc_url: None,
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

/// 与 `wait_ready_and_verify` 相同的健康等待，但每轮同时检查新增日志：
/// 旧版 CLI 不认识 --rc 会立即退出并打印 `unexpected argument`，这里马上
/// 返回明确错误，不空等 30 秒健康超时。
fn wait_ready_and_verify_rc(
    client: &Client,
    distro: &str,
    log_offset: usize,
) -> Result<(u16, String), String> {
    let deadline = Instant::now();
    let port = loop {
        if let Some(port) = probe_running(client, Duration::from_secs(2)) {
            break port;
        }
        if matches!(probe_rc_log(distro, log_offset), RcLogProbe::Unsupported) {
            return Err(RC_UNSUPPORTED_MSG.to_string());
        }
        if deadline.elapsed() >= HEALTH_WAIT_MAX {
            return Err(format!(
                "等待 kimi web 就绪超时（{} 秒）。请检查 WSL 内 ~/.kimi-code/companion-web.log",
                HEALTH_WAIT_MAX.as_secs()
            ));
        }
        std::thread::sleep(HEALTH_POLL_INTERVAL);
    };
    let token = read_verified_token(client, distro, port)?;
    Ok((port, token))
}

/// 结束 web 服务进程并等端口清空（最多 10 秒）。pid 取自实例文件；
/// shutdown API 在 token 不匹配时会失败，直接 kill 最稳。
fn kill_web_and_wait(client: &Client, distro: &str) -> Result<(), String> {
    wsl::run_in_wsl(
        distro,
        "kill $(grep -o '\"pid\":[0-9]*' ~/.kimi-code/server/instances/*.json 2>/dev/null | cut -d: -f2) 2>/dev/null || true",
    )?;

    let deadline = Instant::now();
    while probe_running(client, Duration::from_millis(500)).is_some() {
        if deadline.elapsed() >= Duration::from_secs(10) {
            return Err("kimi web 进程未在 10 秒内退出".to_string());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Ok(())
}

fn store_instance(
    state: &Arc<Mutex<Option<RunningInstance>>>,
    instance: RunningInstance,
) -> Result<(), String> {
    let mut guard = state.lock().map_err(|_| "应用状态锁已损坏".to_string())?;
    *guard = Some(instance);
    Ok(())
}

/// 安装/更新记账 hook 后调用：kimi web 只在启动时加载一次 hooks 配置，
/// 已在运行的实例不会加载新配置（实测确认），因此 web 在跑就重启它让记账
/// 立即生效。只结束 web 服务进程（pid 取自实例文件），TUI 会话不受影响。
/// 重启保持原来的远程控制开关；RC 链接解析失败只记 None，不让重启本身失败。
/// 返回是否执行了重启。
pub fn restart_web_if_running(
    state: &Arc<Mutex<Option<RunningInstance>>>,
    distro: &str,
) -> Result<bool, String> {
    let client = shared_client()?;
    if probe_running(client, Duration::from_millis(800)).is_none() {
        return Ok(false);
    }

    // 旧实例（若由本应用启动/接管）的远程控制开关决定重启参数。
    let remote_control = state
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|inst| inst.remote_control))
        .unwrap_or(false);

    kill_web_and_wait(client, distro)?;

    let offset = if remote_control {
        Some(log_line_offset(distro))
    } else {
        None
    };
    spawn_web(distro, remote_control)?;
    let (port, token) = match offset {
        Some(offset) => wait_ready_and_verify_rc(client, distro, offset)?,
        None => wait_ready_and_verify(client, distro)?,
    };

    // RC 链接解析失败（网络/日志时序）不影响重启本身，例如 hook 安装。
    let rc_url = offset.and_then(|offset| match wait_for_rc_url(distro, offset) {
        RcLinkWait::Found(url) => Some(url),
        _ => None,
    });

    store_instance(
        state,
        RunningInstance {
            distro: distro.to_string(),
            port,
            token,
            remote_control,
            rc_url,
        },
    )?;
    invalidate_probe_cache();
    Ok(true)
}

/// 开关 kimi web 远程控制（`kimi web --rc`）。
///
/// enable=true：已有实例先重启为 --rc 模式，健康后解析云端中继链接；
/// enable=false：在跑则重启为普通模式，否则只清掉状态里的远程控制标记。
pub fn set_remote_control(
    state: &Arc<Mutex<Option<RunningInstance>>>,
    distro: &str,
    enable: bool,
) -> Result<LaunchStatus, String> {
    let client = shared_client()?;
    let running = probe_running(client, Duration::from_millis(800)).is_some();

    if !enable {
        if running {
            kill_web_and_wait(client, distro)?;
            spawn_web(distro, false)?;
            let (port, token) = wait_ready_and_verify(client, distro)?;
            store_instance(
                state,
                RunningInstance {
                    distro: distro.to_string(),
                    port,
                    token,
                    remote_control: false,
                    rc_url: None,
                },
            )?;
            invalidate_probe_cache();
        } else if let Ok(mut guard) = state.lock() {
            if let Some(instance) = guard.as_mut() {
                instance.remote_control = false;
                instance.rc_url = None;
            }
        }
        return Ok(get_status(state, Some(distro)));
    }

    if running {
        kill_web_and_wait(client, distro)?;
    } else {
        let probe = wsl::probe(distro)?;
        if !probe.kimi_installed {
            return Err(format!("发行版 {distro} 中没有找到 kimi CLI（which kimi 为空）"));
        }
    }

    // 先记偏移再启动：只解析本次启动追加的日志，避免误取旧链接。
    let offset = log_line_offset(distro);
    spawn_web(distro, true)?;
    let (port, token) = wait_ready_and_verify_rc(client, distro, offset)?;

    // 健康后最多再等 10 秒：进程可能稍后才打印链接、写二维码。
    let rc_url = match wait_for_rc_url(distro, offset) {
        RcLinkWait::Found(url) => url,
        RcLinkWait::Unsupported => return Err(RC_UNSUPPORTED_MSG.to_string()),
        RcLinkWait::Timeout => {
            // 实例已在运行：先落状态再报错，避免前端显示未运行但端口实际被占用。
            store_instance(
                state,
                RunningInstance {
                    distro: distro.to_string(),
                    port,
                    token,
                    remote_control: true,
                    rc_url: None,
                },
            )?;
            invalidate_probe_cache();
            return Err(RC_LINK_TIMEOUT_MSG.to_string());
        }
    };

    store_instance(
        state,
        RunningInstance {
            distro: distro.to_string(),
            port,
            token,
            remote_control: true,
            rc_url: Some(rc_url),
        },
    )?;
    invalidate_probe_cache();
    Ok(get_status(state, Some(distro)))
}

/// 读取远程控制二维码 PNG（WSL 内 `~/.kimi-code/rc-qrcode.png`），返回 base64。
pub fn rc_qrcode_base64(distro: &str) -> Result<String, String> {
    let out = wsl::run_in_wsl(distro, "base64 -w0 ~/.kimi-code/rc-qrcode.png")
        .map_err(|_| "二维码文件不存在，请先开启远程控制".to_string())?;
    if out.is_empty() {
        return Err("二维码文件不存在，请先开启远程控制".to_string());
    }
    Ok(out)
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
