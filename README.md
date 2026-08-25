# Kimi Companion

Windows 桌面应用：一键启动 / 停止 WSL 中的 `kimi web`，并对 Kimi Code 的 token 用量做本地记账与统计。

## 功能

- **一键启动 / 停止**：从 Windows 侧拉起 WSL 里的 `kimi web`，健康检查通过后自动打开浏览器页面；没有 Kimi Code CLI 会话时同时弹终端启动。停止时优雅关停 web 服务并清剿全部 kimi 进程（含 CLI 会话）
- **token 记账**：通过 Kimi Code 的 hooks（Stop / Interrupt / SessionEnd）把每次 LLM 调用的用量写入 WSL 侧账本 `~/.kimi-code/token-ledger.jsonl`，应用后台每 30 秒增量同步进本地 SQLite（幂等去重，支持历史回填）
- **统计看板**：今日 / 本周 / 本月 / 总计用量、缓存命中率、近 30 天趋势图、近一年热力图、按模型占比
- **费用估算**：按模型单价（自行填写，localStorage 保存）折算人民币花费
- **套餐配额**：透传 kimi web 的 `/api/v1/oauth/usage` 展示配额窗口

## 运行环境

- Windows 10 / 11（依赖系统 WebView2 运行时）
- WSL2 发行版，内部已安装 [Kimi Code CLI](https://www.kimi.com/)（`kimi` 命令可用）

## 安装与使用

1. 从 [Releases](../../releases) 下载 `Kimi Companion_x.y.z_x64-setup.exe`（NSIS 安装包）并安装
2. 启动应用，选择 WSL 发行版
3. 首次使用点「一键安装记账」（向 WSL 的 `~/.kimi-code/` 写入 hook 脚本并登记配置，含自动备份与历史回填）
4. 点「启动 kimi web」即可

数据位置：SQLite 在 `%APPDATA%/com.kimicompanion.app/app.db`；WSL 侧账本在 `~/.kimi-code/token-ledger.jsonl`。

## 开发

技术栈：Tauri 2（Rust 后端）+ React 18 + TypeScript + Vite + Tailwind CSS + TanStack Query + recharts + rusqlite。

```bash
npm install
npm run tauri dev    # 开发调试
npm run tauri build  # 产出安装包（src-tauri/target/release/bundle/）
```

手动安装记账 hook（不经过应用）：`bash hook/install.sh --backfill`。

### 目录结构

- `src/` — 前端（components/{home,launch,stats,heatmap}）
- `src-tauri/src/` — Rust 后端（services: wsl / launcher / ledger / hook / stats / http；database: SQLite DAO 与迁移；commands: IPC 命令层）
- `hook/` — 记账 hook 脚本（应用的 `include_str!` 与 `install.sh` 共用此唯一真源）
- `scripts/bump-version.mjs` — 版本号同步脚本

### 发版流程

```bash
node scripts/bump-version.mjs patch   # 或 minor / major / x.y.z，三处版本号同步
# 更新 CHANGELOG.md，提交，打 tag：
git tag v1.0.1
npm run tauri build                   # Windows 侧产出 msi / nsis 安装包
# 在 GitHub Releases 用 tag 发布并上传安装包
```

## 版本管理

- 语义化版本，`package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` 三处保持一致（用上面的脚本改）
- 每个发布版本对应一个 `v*` git tag 和 GitHub Release
- 变更记录见 [CHANGELOG.md](CHANGELOG.md)
