# 更新日志

本文件记录 AgentShield MCP 的版本变更。格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### 进行中（v0.1 · MVP）

- MCP Proxy stdio 转发主循环：拉起上游 server，`tools/call` 拦截、其余透传，
  阻止时回标准 JSON-RPC 错误，客户端 EOF 后优雅关闭上游并回收
- `proxy start` 可真实转发（支持 `--server` 读配置或 `--command` 直接指定上游）
- 审计结果回填：放行的 tools/call 按 JSON-RPC id 关联上游响应，把真实
  result/error 一并写入审计；上游无响应时兜底补记
- 审计双写：JSONL（崩溃安全）+ SQLite（结构化查询）
- `report generate` 生成审计报告，支持 JSON / Markdown / HTML
- 确认结果持久化：选「始终允许 / 永久拉黑」会写入 `.agentshield/decisions.json`，
  下次同一操作（按 server + 工具 + 规范化目标匹配）直接放行 / 拦截，记忆优先于策略；
  新增 `memory list` 查看
- 上游传输抽象为 `Transport`：除 stdio 子进程外，新增 Streamable HTTP 上游
  （POST + JSON/SSE 响应、自动捕获并复用会话 id）；`mcp add` / `proxy start` 支持 `--url`
- HTTP 上游会话建立后维持 GET SSE 长连接，接收并透传上游主动发起的消息
  （sampling/elicitation 请求、进度与资源更新通知等）
- 审计查询支持按 server、时间范围（`--since` / `--until`，接受日期或 RFC3339）过滤
- 多 MCP Server 聚合（`proxy start --all`）：合并 `tools/list`（工具名加 `server__` 前缀）、
  按前缀路由 `tools/call`、各上游启动时自动 prime initialize，拦截与审计照常
- Web 监控仪表盘：`agentshield dashboard` 提供本地只读 JSON API 并托管 Vue 前端
  （总览 / 实时事件 / 审计日志 / MCP Server / 确认记忆 / 报告），技术栈
  Vue 3 + TS + Vite + Ant Design Vue + ECharts，结构预留 Tauri
- `agentshield demo` 现在会把演示事件写入审计，便于随后用仪表盘 / 报告查看
- 重构：`DecisionMemory` 下沉到 `agentshield-core`，仪表盘后端抽成独立 lib
  `agentshield-dashboard`，供 CLI 与桌面外壳共用
- Tauri 桌面外壳（`desktop/src-tauri`）：进程内启动仪表盘服务，用原生窗口加载，
  逻辑零重写；独立于 Rust 工作区（`cargo` 默认不构建它）
- `init` / `mcp add` / `mcp list` / `audit list`（按等级过滤、读 SQLite）命令落地
- 终端确认走控制台设备（`/dev/tty`、`CONIN$/CONOUT$`），不污染 MCP 通道
- 内置风险规则与风险评分引擎
- `agentshield demo` 一键演示
- 示例策略文件与开源文档

### 待办

- 仪表盘内的待确认操作（Approvals）与弹窗确认（需代理↔面板双向通道）
- 桌面外壳打包产物的图标与签名、自动随项目目录监听

## [0.0.0] - 2026-06-19

### 新增

- 项目立项，完成需求文档、架构设计、各模块设计文档。
- 初始化 Cargo workspace 与基础 crate 结构。
- 确定技术选型：Rust + tokio + clap + serde + SQLite。

---

### 版本规划

| 版本 | 重点 |
|---|---|
| v0.1 | MCP Proxy、风险评分、CLI 确认、JSONL 审计、Demo |
| v0.2 | YAML 策略引擎、白/黑名单、MCP Server 权限等级 |
| v0.3 | Tauri 桌面端、实时事件流、风险看板 |
| v0.4 | Shell Guard：危险命令识别与确认 |
| v0.5 | File Guard：文件变更监听、敏感文件保护 |
| v1.0 | 完整版：DB Guard、审计报告、插件系统、GitHub Action |
