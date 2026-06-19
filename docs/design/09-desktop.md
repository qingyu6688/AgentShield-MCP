# 设计 · agentshield-desktop

桌面端监控面板。可视化展示实时事件、风险分布、MCP server 权限、策略和审计报告，并支持弹窗确认。**v0.3 起开发，MVP 不含**。技术选型：Tauri + Vue 3 + TypeScript + Ant Design Vue + ECharts。

## 职责

- 实时展示工具调用事件流。
- 风险统计看板。
- 管理 MCP server 与权限等级。
- 弹窗确认高危操作（替代/补充终端确认）。
- 查询审计日志、导出报告。

## 形态

```text
desktop/
├── src/              # Vue 3 前端
│   ├── api/          # 与后端通信（Tauri command / WebSocket）
│   ├── pages/        # Dashboard / LiveEvents / Servers / Policies / Approvals / Audit / Reports / Settings
│   ├── components/
│   ├── stores/       # Pinia
│   └── router/
├── src-tauri/        # Rust 侧，复用 agentshield-* crate
└── package.json
```

Tauri 后端直接复用现有 crate（audit 查询、policy 读写、config 管理），前端通过 Tauri command 调用；实时事件用 WebSocket 推送。

## 页面

| 页面 | 内容 | 需求 |
|---|---|---|
| Dashboard | 今日调用数、高危数、被阻止数、活跃 server 数、最近 10 条风险事件、风险等级分布饼图 | DESKTOP-002 |
| Live Events | 实时事件流，刷新延迟 < 1s | DESKTOP-001 |
| MCP Servers | server 列表、权限等级、白/黑名单编辑 | DESKTOP-003 |
| Policies | 策略规则查看/编辑 | DESKTOP-004 |
| Approvals | 待确认操作，弹窗 allow/deny | DESKTOP-005 |
| Audit Logs | 按时间/等级查询 | DESKTOP-006 |
| Reports | 导出 JSON/MD/HTML | DESKTOP-007 |
| Settings | 暗色模式等 | DESKTOP-008 |

## 实时通道

代理侧增加一个可选的事件广播：每条审计记录除写库外，再推到本地 WebSocket（仅监听 127.0.0.1）。桌面端订阅，实现 < 1s 刷新（NF-003）。该通道默认只本地，不对外暴露（SEC-001）。

## 确认协同

终端确认与桌面确认二选一或并存：

- proxy 的 `Approver` 可切换为 `DesktopApprover`，把确认请求经 WebSocket 推给桌面端，等用户在弹窗里裁决后回传。
- 都不可用时按 `on_timeout` 兜底。

## UI 规范

遵循全局 UI 规范：布局贴合真实运维场景，不堆砌卡片/渐变；空/加载/错误状态完整；支持暗色模式。

## 相关需求

7.12 DESKTOP-001 ~ DESKTOP-008；NF-003。
