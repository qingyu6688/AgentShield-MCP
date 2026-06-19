# 设计 · agentshield-cli

命令行入口，也是顶层装配者。把 core / proxy / policy / risk / rules / audit 组装成可用的二进制，并实现终端确认。基于 `clap`。

## 职责

- 提供所有子命令（init / mcp / proxy / audit / report / policy / demo / version 等）。
- 装配各引擎，给 proxy 注入 `DecisionMaker` / `Approver` / `AuditSink`。
- 实现 `CliApprover`（见 [06-approval](06-approval.md)）。

## 模块结构

```text
agentshield-cli/src/
├── main.rs
├── cli.rs          # clap 定义
├── commands/
│   ├── init.rs
│   ├── mcp.rs      # add / list / remove
│   ├── proxy.rs    # start
│   ├── audit.rs    # list
│   ├── report.rs   # generate
│   ├── policy.rs   # test
│   ├── config.rs   # check
│   └── demo.rs
├── wiring.rs       # 组装 DecisionMaker = policy + risk + rules
├── approver.rs     # CliApprover
└── error.rs        # 用 anyhow
```

## 命令一览

```bash
agentshield init                         # 生成 .agentshield/{config,policy}.yaml + audit.db
agentshield mcp add <name> --command <c> [--args a,b] [--env K=V] [--trust N]
agentshield mcp list
agentshield mcp remove <name>
agentshield proxy start [--server <name> | --all]
agentshield audit list [--level <l>] [--since <t>] [--limit N]
agentshield report generate [--format json|markdown|html] [-o <file>]
agentshield policy test [--type <t>] [--command <c>] [--path <p>] [--sql <s>]
agentshield config check
agentshield demo
agentshield --version
```

每个子命令都带 `--help`（MAINT-004）。

| 命令 | 需求 |
|---|---|
| init | CLI-001 |
| mcp add/list | CLI-002 / CLI-003 |
| proxy start | CLI-004 |
| audit list | CLI-005 |
| report generate | CLI-006 |
| policy test | CLI-007 |
| demo | CLI-008 |
| --version | CLI-009 |
| config check | CLI-010 |

## 装配（wiring）

把策略 + 风险 + 规则组合成一个 `DecisionMaker` 给 proxy：

```rust
struct AppDecisionMaker {
    risk: RiskEngine,
    policy: PolicyEngine,
    rules: RuleRegistry,
}

#[async_trait::async_trait]
impl DecisionMaker for AppDecisionMaker {
    async fn decide(&self, call: &ToolCall) -> Decision {
        let hits = self.rules.evaluate_all(call);
        let risk = self.risk.assess(call, &ctx, &hits);   // rules 命中参与计分
        self.policy.decide(call, risk)                    // 合成最终动作
    }
}
```

`proxy start` 时：加载 config → 建 RiskEngine / PolicyEngine / RuleRegistry → 建 AuditWriter → 建 CliApprover → 启动 gateway。

## init 行为

- 在当前目录建 `.agentshield/`。
- 写入 `config.yaml`（含示例注释）、`policy.yaml`（复制 `policies/default.yaml`）。
- 建空的 `audit.db`（建表）。
- 配置文件权限设为当前用户可读写（SEC-006）。
- 已存在则提示，不覆盖（除非 `--force`）。

## demo

不需要真实 server / 客户端。内置一组预设的危险 `ToolCall`，跑完整决策链路并把结果打到终端，最后写一条条审计。用于 README 演示和快速自检（CLI-008 / MVP-007）。

## 错误处理

二进制入口用 `anyhow`，给用户的报错友好、可理解（不暴露原始堆栈，SEC：生产不暴露堆栈）。退出码：成功 0，被 block 的 demo/test 用非 0 表示“拦截发生”。

## 测试要点

- 各子命令参数解析。
- init 幂等与权限设置。
- policy test / demo 的输出符合预期。
- 端到端：mock 上游 server，跑一次 allow / confirm / block。

## 相关需求

7.11 CLI-001 ~ CLI-010；MVP-005 / 007 / 008。
