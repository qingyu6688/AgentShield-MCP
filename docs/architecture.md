# 架构设计

本文说明 AgentShield MCP 的整体结构、数据流和模块划分。模块级的详细设计在 [docs/design/](design/)。

## 1. 设计目标

一句话：**在 AI Agent 的工具调用真正执行之前介入，并完整记录。**

三个不可妥协的约束：

- **本地优先**：默认不联网、不上传任何用户数据。
- **低开销**：代理层自身延迟尽量控制在 100ms 内（不含真实工具执行时间，NF-001）。
- **可旁路失败**：AgentShield 出问题时要可被快速摘除，不能把用户的开发流程彻底卡死。

## 2. 分层架构

```text
┌──────────────────────────────────────────────┐
│              AI Coding Clients                  │
│        Cursor / Codex / Gemini CLI           │
└─────────────────────┬───────────────────────────┘
                      │ MCP (stdio / HTTP)
                      ▼
┌──────────────────────────────────────────────┐
│                  Gateway 层                     │
│   MCP Proxy · (后续) Command Proxy · File Watcher │
└─────────────────────┬───────────────────────────┘
                      ▼
┌──────────────────────────────────────────────┐
│              Security Decision 层               │
│   Policy Engine · Risk Engine · Approval         │
└─────────────────────┬───────────────────────────┘
                      ▼
┌──────────────────────────────────────────────┐
│                Execution 层                     │
│   Real MCP Server · Sandbox · Shell · DB · HTTP  │
└─────────────────────┬───────────────────────────┘
                      ▼
┌──────────────────────────────────────────────┐
│                  Audit 层                       │
│   SQLite · JSONL · Dashboard · Reports           │
└──────────────────────────────────────────────┘
```

各层职责：

| 层 | 职责 | 对应 crate |
|---|---|---|
| Gateway | 接收 AI 请求，解析工具调用，转发到真实 server | `agentshield-proxy` |
| Security Decision | 评分、匹配策略、触发确认 | `agentshield-risk` / `agentshield-policy` / `agentshield-cli`(确认 UI) |
| Execution | 真正执行被放行的调用 | 真实 MCP Server / 后续 sandbox |
| Audit | 记录事件、生成报告 | `agentshield-audit` |
| 公共能力 | 类型、配置、脱敏、错误 | `agentshield-core` |

## 3. 调用时序

以一次 `tools/call` 为例：

```text
AI Client ──tools/call──▶ Proxy
                          │ 1. 解析：工具名 / 参数 / 来源 / 上下文
                          ▼
                       构造 ToolCall（core 类型）
                          │
                          ├──▶ Policy Engine  匹配规则 → 候选动作
                          ├──▶ Risk Engine    打分 → score / level / reasons
                          ▼
                       Decision 合成（取更严格者）
                          │
              ┌───────────┼───────────┬────────────┐
            allow        log        confirm        block
              │           │            │             │
              │           │      Approval(CLI)        │
              │           │       y/a/n/b             │
              ▼           ▼            ▼              ▼
           转发        转发+记录   同意→转发/拒绝   不转发
              │
              ▼
        Real MCP Server ──result──▶ Proxy ──▶ AI Client
              │
              ▼
        Audit 写入 security_events（含 result、decision、reason）
```

**决策合成原则**：策略引擎给出的动作和风险引擎给出的建议动作中，**取更严格的一个**。例如风险分只有 40（建议 log），但策略命中 `block-env-read`，最终就是 block。

## 4. crate 划分

采用 Cargo workspace，按职责拆 crate，公共类型下沉到 `core`：

```text
crates/
├── agentshield-core      # 核心类型、配置、错误、脱敏；不依赖其他业务 crate
├── agentshield-proxy     # MCP 协议解析、stdio/HTTP 传输、转发与拦截
├── agentshield-policy    # YAML 策略加载、匹配引擎、热更新
├── agentshield-risk      # 风险评分引擎、内置评分维度
├── agentshield-rules     # 内置规则库（shell/file/db），独立维护
├── agentshield-audit     # SQLite + JSONL 存储、查询、报告生成
└── agentshield-cli       # 命令行入口、确认 UI，组装上面所有 crate
```

依赖方向（单向，避免环）：

```text
core ◀── policy ◀──┐
core ◀── risk   ◀──┤
core ◀── rules  ◀──┤
core ◀── audit  ◀──┤
core ◀── proxy  ◀──┤
                   └── cli（顶层组装）
```

`rules` 依赖 `core`、被 `risk` / `policy` 使用；`cli` 依赖所有人。任何 crate 都不反向依赖 `cli`。

## 5. 核心数据模型

定义在 `agentshield-core`，是各模块之间传递的统一语言。

```rust
/// 一次被拦截的工具调用，所有决策与审计都围绕它展开
pub struct ToolCall {
    pub id: String,            // 事件唯一 id（uuid）
    pub session_id: String,    // 会话 id
    pub client_name: String,   // 来源客户端，如 "Codex CLI"
    pub server_name: String,   // 目标 MCP server 名
    pub tool_name: String,     // 工具名，如 "fs.read_file"
    pub event_type: EventType, // file.read / shell.exec / db.query ...
    pub target: Option<String>,// 目标资源：路径 / 命令 / SQL
    pub arguments: serde_json::Value,
}

pub enum EventType {
    FileRead, FileWrite, FileDelete, FileRename,
    ShellExec, DbQuery, NetworkRequest, McpToolCall, Other,
}

/// 风险评分结果
pub struct RiskAssessment {
    pub score: u8,             // 0-100
    pub level: RiskLevel,      // Low / Medium / High / Critical
    pub reasons: Vec<String>,
    pub recommended_action: Action,
}

pub enum RiskLevel { Low, Medium, High, Critical }

/// 最终决策动作
pub enum Action { Allow, Log, Confirm, Block, Sandbox }

/// 合成后的决策
pub struct Decision {
    pub action: Action,
    pub risk: RiskAssessment,
    pub matched_rule: Option<String>, // 命中的策略规则名
    pub reason: String,
}
```

`EventType` 由 Proxy 在解析阶段根据工具名/参数推断（详见 [mcp-proxy.md](../docs/mcp-proxy.md)）。

## 6. 风险等级与默认动作

| 分数 | 等级 | 默认动作 |
|---|---|---|
| 0–29 | Low | Allow |
| 30–59 | Medium | Log |
| 60–79 | High | Confirm |
| 80–100 | Critical | Block / Confirm |

策略可以覆盖默认动作（取更严格者）。

## 7. 存储与审计

- **SQLite**：结构化查询（按时间、按风险等级、按 server），表结构见 [docs/design/08-audit.md](design/08-audit.md)。
- **JSONL**：追加写，崩溃安全、易被外部工具消费、方便流式导出。
- 两者并行写：JSONL 保证“一定记下来”，SQLite 保证“查得动”。
- 敏感字段在写入前由 `core` 的脱敏器处理。

## 8. 配置布局

`agentshield init` 在项目下生成：

```text
.agentshield/
├── config.yaml    # 全局配置：监听方式、MCP server 列表、确认超时等
├── policy.yaml    # 策略规则
└── audit.db       # SQLite 审计库（JSONL 同目录 audit.jsonl）
```

## 9. 性能与可靠性

- 审计写入走独立任务/通道，**不阻塞**转发主路径（NF-002）。
- 风险评分是纯函数式、无 IO，单次调用微秒级。
- 策略文件支持热更新（watch 文件变更，POLICY-009）。
- 真实 server 不可用时，Proxy 返回明确错误而非静默吞掉。

## 10. 扩展点

- **规则插件化**：`rules` crate 通过 trait 暴露 `Rule`，第三方可注册自定义规则（MAINT-006）。
- **传输适配**：Proxy 的 `Transport` trait 抽象 stdio / HTTP，新增传输只需实现 trait。
- **存储后端**：Audit 的 `AuditSink` trait 抽象 JSONL / SQLite，便于后续接其他后端。

各扩展点的 trait 定义见对应模块设计文档。
