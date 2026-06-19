# 模块设计

本目录是 AgentShield MCP 各模块的详细设计，配合 [架构总览](../architecture.md) 阅读。每个 crate 一篇，说明职责、对外类型/trait、内部结构和关键取舍。

| 文档 | crate | 职责 |
|---|---|---|
| [01-core](01-core.md) | `agentshield-core` | 核心类型、配置、错误、脱敏 |
| [02-proxy](02-proxy.md) | `agentshield-proxy` | MCP 协议解析、传输、转发与拦截 |
| [03-policy](03-policy.md) | `agentshield-policy` | YAML 策略加载、匹配、热更新 |
| [04-risk](04-risk.md) | `agentshield-risk` | 风险评分引擎 |
| [05-rules](05-rules.md) | `agentshield-rules` | 内置规则库（shell/file/db） |
| [06-approval](06-approval.md) | （cli 内） | 用户确认流程与交互 |
| [07-cli](07-cli.md) | `agentshield-cli` | 命令行入口，组装全部模块 |
| [08-audit](08-audit.md) | `agentshield-audit` | 审计存储、查询、报告 |
| [09-desktop](09-desktop.md) | `agentshield-desktop` | 桌面端监控面板（v0.3+） |

## 依赖关系

```text
            ┌──────────────── core ────────────────┐
            │        │        │        │        │   │
          rules    policy    risk    audit    proxy │
            │        │        │        │        │   │
            └────────┴────┬───┴────────┴────────┘   │
                          │                          │
                         cli ─────────────────────── ┘
                          │
                      desktop（经 cli/audit 提供的接口）
```

- `core` 不依赖任何业务 crate，是公共词汇表。
- `rules` 依赖 `core`，被 `risk` / `policy` 使用。
- `cli` 是顶层装配者，依赖所有人；没人反向依赖 `cli`。

## MVP 范围

v0.1 落地：`core`、`proxy`、`risk`、`rules`、`audit`、`cli`（含 approval）。`policy` 在 v0.2 完整化（MVP 先内置默认规则），`desktop` v0.3 起。
