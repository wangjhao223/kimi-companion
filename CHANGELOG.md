# Changelog

本项目遵循语义化版本（Semantic Versioning）。

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
