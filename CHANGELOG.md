# Changelog

本项目遵循语义化版本（Semantic Versioning）。

## [1.0.1] - 2026-08-25

### 修复

- 修复「kimi 已安装」「Kimi Code CLI 运行中」状态灯恒灭、且无法一键启动 kimi web 的根因：wsl.exe 会把 `--` 之后的参数重新拼接后交给 /bin/bash -c 二次解析，探测脚本里的 `$i`/`$t` 被外层 shell 提前展开为空，导致探测恒返回未安装、启动流程在「没有找到 kimi CLI」处中止。现统一改为 base64 编码传输命令、WSL 内解码执行，规避双层 shell 展开
- 一键安装记账后，若 kimi web 正在运行则自动重启它（只结束 web 进程，不动 TUI 会话）：kimi web 只在启动时加载一次 hooks 配置，旧进程不会加载新安装的 hook，导致 token 计数静默失效
- WSL 发行版列表获取失败（如 wsl.exe 偶发失败、VM 未启动）时不再永久卡在「未检测到发行版」：每 3 秒自动重试，并在面板上显示具体错误
- 启动 kimi web / TUI 时固定以 WSL 用户主目录（`~`）为起始目录，不再落在 Windows 用户目录（`/mnt/c/Users/<用户>`）
- token 计数改为事件驱动：hook 写完账本后主动 POST 应用的联动端口（127.0.0.1:51999，仅 loopback），应用立即同步一轮，计数从"回答结束后最多约 1 分钟"提速到秒级；30 秒轮询保留为兜底（NAT 模式 / 应用未运行时自动回退）

## [1.0.0] - 2026-08-25

首个正式发布。

### 功能

- 一键启动 / 停止 WSL 中的 kimi web：端口区间健康探测复用已有实例、隐藏长驻 wsl.exe 承载 web 进程、无 TUI 会话时自动弹出终端启动 Kimi Code CLI；停止时 shutdown API 优雅关停 + 清剿全部 kimi 进程
- token 记账 hook：挂在 Kimi Code 的 Stop / Interrupt / SessionEnd 事件，解析 wire.jsonl 的 usage.record 事件写入 JSONL 账本，支持历史回填
- 账本同步：后台线程每 30 秒经 UNC 路径按字节偏移增量同步进 SQLite（WAL，INSERT OR IGNORE 幂等）
- 统计看板：今日 / 本周 / 本月 / 总计用量汇总、缓存命中率、token 单位 k/M 切换
- 近 30 天用量趋势图、近一年用量热力图（总量 / 输出 / 缓存三个维度）
- 按模型用量占比、按自填单价估算人民币费用、套餐配额窗口展示
- 自定义标题栏与紧凑暗色界面

### 性能

- 状态轮询合并为单次 WSL 探测（10 秒 TTL 缓存，start/stop 后主动失效），wsl.exe 进程拉起从每 2.5 秒 3 次降到每 10 秒 1 次
- reqwest HTTP 客户端进程级复用
- 趋势查询走 ts 索引范围扫描，避免全表逐行求值
- 待机 CPU ≈ 0%，主进程私有内存约 7 MB（其余为 WebView2 运行时开销）
