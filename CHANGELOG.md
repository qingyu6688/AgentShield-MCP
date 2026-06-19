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
- `init` / `mcp add` / `mcp list` / `audit list`（按等级过滤、读 SQLite）命令落地
- 终端确认走控制台设备（`/dev/tty`、`CONIN$/CONOUT$`），不污染 MCP 通道
- 内置风险规则与风险评分引擎
- `agentshield demo` 一键演示
- 示例策略文件与开源文档

### 待办

- 审计按时间范围 / server 维度查询
- 确认结果的白/黑名单持久化
- MCP SSE / Streamable HTTP 传输

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
