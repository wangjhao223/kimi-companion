//! 记账 hook 的检测与安装（目标：WSL 内的 ~/.kimi-code/）。
//!
//! 脚本内容以 include_str! 内嵌进二进制（唯一真源在仓库根目录 hook/，与手动
//! 安装脚本 install.sh 共用同一份，避免两处副本漂移），安装时 base64 传输到
//! WSL 侧解码写入，避免多层 shell 引号转义问题。整体幂等：脚本直接覆盖写，
//! hooks 配置只在缺失时追加（追加前备份 config.toml），最后跑一次 --backfill
//! 补历史数据。

use base64::Engine;

use crate::services::wsl;

const HOOK_SH: &str = include_str!("../../../hook/companion-hook.sh");
const HOOK_PY: &str = include_str!("../../../hook/companion-hook.py");

/// 追加到 config.toml 的 hooks 配置，与 hook/install.sh 写入的内容一致。
const HOOKS_TOML: &str = r#"
# Kimi Companion: token bookkeeping (https://github.com/kimi-companion)
[[hooks]]
event = "Stop"
command = "bash ~/.kimi-code/companion-hook.sh"
timeout = 10

[[hooks]]
event = "Interrupt"
command = "bash ~/.kimi-code/companion-hook.sh"
timeout = 10

[[hooks]]
event = "SessionEnd"
command = "bash ~/.kimi-code/companion-hook.sh"
timeout = 10
"#;

/// 检测 hook 是否已安装：config.toml 含 companion-hook 条目且 .sh 存在。
///
/// 命令恒以 0 退出（`; true` 兜底），用输出是否 yes 表达结果，
/// 避免把「未安装」（grep 退出码 1）和「WSL 执行失败」混为一谈。
pub fn check_installed(distro: &str) -> Result<bool, String> {
    let out = wsl::run_in_wsl(
        distro,
        "grep -q companion-hook ~/.kimi-code/config.toml 2>/dev/null \
         && test -f ~/.kimi-code/companion-hook.sh \
         && echo yes; true",
    )?;
    Ok(out == "yes")
}

/// 一键安装：写两个脚本 → 配置 hooks（缺失才追加）→ 跑一次 backfill。
pub fn install(distro: &str) -> Result<(), String> {
    let b64 = base64::engine::general_purpose::STANDARD;

    // 1. 写入两个脚本（覆盖写，天然幂等），.sh 加可执行权限
    let py_b64 = b64.encode(HOOK_PY);
    let sh_b64 = b64.encode(HOOK_SH);
    wsl::run_in_wsl(
        distro,
        &format!(
            "mkdir -p ~/.kimi-code \
             && echo '{py_b64}' | base64 -d > ~/.kimi-code/companion-hook.py \
             && echo '{sh_b64}' | base64 -d > ~/.kimi-code/companion-hook.sh \
             && chmod +x ~/.kimi-code/companion-hook.sh"
        ),
    )
    .map_err(|e| format!("写入 hook 脚本失败: {e}"))?;

    // 2. config.toml 未配置时才追加，追加前留时间戳备份
    let hooks_b64 = b64.encode(HOOKS_TOML);
    wsl::run_in_wsl(
        distro,
        &format!(
            "if ! grep -q companion-hook ~/.kimi-code/config.toml 2>/dev/null; then \
               cp ~/.kimi-code/config.toml ~/.kimi-code/config.toml.$(date +%Y%m%d-%H%M%S).bak 2>/dev/null || true; \
               echo '{hooks_b64}' | base64 -d >> ~/.kimi-code/config.toml; \
             fi"
        ),
    )
    .map_err(|e| format!("更新 config.toml 失败: {e}"))?;

    // 3. 历史数据回填（脚本 fail-open，失败不阻塞安装结果，但把错误透出来便于排查）
    wsl::run_in_wsl(distro, "python3 ~/.kimi-code/companion-hook.py --backfill")
        .map_err(|e| format!("hook 已安装，但历史回填失败: {e}"))?;

    Ok(())
}
