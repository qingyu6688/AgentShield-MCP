# AgentShield 监控面板（Web 仪表盘）

本地优先的可视化面板：总览、实时事件、审计日志、MCP Server、确认记忆、报告导出。
技术栈 Vue 3 + TypeScript + Vite + Ant Design Vue + ECharts。

后端是 `agentshield dashboard`（CLI 内置，tiny_http），提供只读 JSON API 并托管本目录的构建产物。
数据全部来自本机 `.agentshield/`（审计库、配置、确认记忆），不出本机。

> 当前为 Web 仪表盘形态，目录与接口按可套 Tauri 外壳的方式组织（前端经 `/api` 访问，
> 后续可由 Tauri command 复用同一套查询）。

## 开发

```bash
# 1) 起后端（另一个终端），先确保有审计数据：agentshield demo
agentshield dashboard            # 默认 127.0.0.1:8787

# 2) 起前端开发服务器（已把 /api 代理到 8787）
cd desktop
npm install
npm run dev                      # http://localhost:5173
```

## 构建并由 CLI 托管

```bash
cd desktop && npm run build      # 产出 desktop/dist
agentshield dashboard            # 浏览器打开 http://127.0.0.1:8787
```

## 页面

| 页面 | 内容 |
|---|---|
| 总览 | 今日调用 / 高危 / 被阻止 / 活跃 server 指标卡 + 风险等级分布饼图 + 最近 10 条 |
| 实时事件 | 每 1.5s 轮询，最新事件在前，可暂停 |
| 审计日志 | 按等级 / server / 时间范围过滤，点详情看脱敏参数与结果 |
| MCP Server | 已配置上游、传输方式、信任等级、启用状态 |
| 确认记忆 | 始终允许 / 永久拉黑名单 |
| 报告 | 生成 Markdown / JSON / HTML 报告并下载 |

## 接口

仪表盘后端提供：`/api/summary`、`/api/events`、`/api/servers`、`/api/memory`、`/api/report`。

## Tauri 桌面外壳

`src-tauri/` 是一个 Tauri v2 外壳：**不重写任何逻辑**，而是在应用进程内启动同一套
`agentshield-dashboard` 服务（127.0.0.1:8787），再用原生窗口加载它。前端与浏览器访问完全一致。

```bash
cd desktop
npm install                # 含 @tauri-apps/cli
npm run build              # 先产出 dist（窗口加载的服务会托管它）
npm run tauri:build        # 打包成安装包 / 可执行文件
# 或开发：npm run tauri:dev
```

- 监控的项目目录默认取启动时的工作目录，可用环境变量 `AGENTSHIELD_HOME` 指定。
- `src-tauri` 独立于 Rust 工作区（根 `Cargo.toml` 的 `exclude`），不会被普通 `cargo build` / `cargo test` 牵连。
- 运行 Tauri 需要本机具备 Tauri 前置环境（Rust + 平台 WebView：Windows 为 WebView2）。
- `icons/` 内为占位图标，正式发布可用 `npm run tauri icon <你的图.png>` 一键生成整套。
