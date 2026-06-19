# 设计 · 用户确认（approval）

确认流程。当决策为 `confirm` 时，向用户征求裁决。MVP 阶段实现在 `agentshield-cli` 内（终端确认），桌面端确认在 v0.3 由 desktop 提供。这里定义通用契约。

## 职责

- 在高危操作执行前，把调用、风险、原因清晰呈现给用户。
- 收集用户裁决：允许一次 / 始终允许 / 拒绝 / 拉黑 / 查看详情。
- 记住“始终允许 / 拉黑”的选择，影响后续同类调用。
- 支持确认超时（默认动作可配）。

## 契约

proxy 通过 `Approver` trait 触发确认（见 [02-proxy](02-proxy.md)）：

```rust
#[async_trait::async_trait]
pub trait Approver: Send + Sync {
    async fn approve(&self, call: &ToolCall, decision: &Decision) -> ApprovalResult;
}

pub enum ApprovalResult {
    AllowOnce,
    AllowAlways,   // 写入白名单
    Deny,
    BlockForever,  // 写入黑名单
    TimedOut,      // 超时，按配置兜底
}
```

## 终端实现（CliApprover）

呈现格式：

```text
  AgentShield · 需要确认

  来源     Codex CLI
  操作     shell.exec
  命令     rm -rf dist
  风险     High  82/100
  原因     递归删除操作，目标在项目目录内

  [y] 允许一次   [a] 始终允许   [n] 拒绝   [b] 永久拉黑   [d] 查看详情
  >
```

- `[d]` 查看详情：展开完整参数（脱敏后）、命中规则、各风险维度贡献。
- 输入读取要在确认期间独占终端，避免和客户端 stdio 抢输入——MVP 用独立的 tty / stderr 交互通道（见“注意事项”）。

## 记忆决策

- `AllowAlways` / `BlockForever` 持久化到 `.agentshield/` 下的白/黑名单（或写回 policy）。
- 匹配键：`(server_name, tool_name, 规范化 target)`，target 做规范化（路径绝对化、命令去多余空白），避免“同一操作不同写法”重复打扰。
- 下次命中白名单直接 Allow，命中黑名单直接 Block，不再询问。

## 超时

```yaml
# config.yaml
approval:
  timeout_secs: 60
  on_timeout: deny      # deny | allow（默认 deny，更安全）
```

超时返回 `TimedOut`，proxy 按 `on_timeout` 兜底，并在审计标注超时。

## 审计

每次确认结果写入 `approvals` 表（event_id、user_decision、remember、created_at），并回填到对应 `security_events`（APPROVAL-008）。

## 注意事项

stdio 模式下，客户端和 AgentShield 共用进程的 stdin/stdout 传 MCP 消息，**不能**直接用 stdout 打确认 UI（会污染协议）。方案：

- 确认 UI 走 **stderr** 或单独打开的控制终端。
- 若运行在无人值守环境（无 tty），自动按 `on_timeout` 处理或要求策略改为非交互（见 [faq](../faq.md)）。

## 测试要点

- 各裁决分支的行为。
- 白/黑名单记忆与匹配键规范化。
- 超时兜底。
- 详情展开内容脱敏正确。

## 相关需求

7.8 APPROVAL-001 ~ APPROVAL-008。
