# MCP 代理

AgentShield 的核心是一个 MCP 代理：对 AI 客户端它伪装成一个 MCP Server，对真实 MCP Server 它又是一个客户端。所有 `tools/call` 都从中间穿过，于是有了拦截点。

## 1. 位置

```text
AI Client  ──(MCP)──▶  AgentShield Proxy  ──(MCP)──▶  Real MCP Server
           ◀──────────                    ◀──────────
```

客户端以为自己在直连一个普通的 MCP Server，实际连的是 AgentShield。

## 2. 处理的 MCP 方法

| 方法 | 处理方式 |
|---|---|
| `initialize` | 透传，必要时改写 serverInfo 标识为 AgentShield 包装 |
| `tools/list` | 转发到真实 server，**可按权限隐藏被禁用工具**，可在描述里加风险提示 |
| `tools/call` | **核心拦截点**：解析 → 评分 → 策略 → 决策 → 放行/确认/阻止 |
| `resources/*`、`prompts/*` | MVP 阶段透传，记录调用 |
| 其他通知 | 透传 |

## 3. tools/call 拦截流程

```text
收到 tools/call
   │
   ▼
解析为 core::ToolCall
   ├─ tool_name           工具名
   ├─ arguments           原始参数（json）
   ├─ event_type          推断：file.read / shell.exec / db.query ...
   ├─ target              提取：路径 / 命令 / SQL
   └─ client / server     来源与目标
   │
   ▼
Risk Engine 打分 + Policy Engine 匹配 → Decision
   │
   ├─ allow / log   →  转发；log 额外写审计
   ├─ confirm       →  调用 Approval，等用户；同意则转发，拒绝则返回 MCP error
   └─ block         →  不转发，返回结构化 MCP error（带原因）
   │
   ▼
（放行时）转发到真实 server，拿到 result
   │
   ▼
写审计：security_events（含 result / decision / reason）
   │
   ▼
把 result 返回给客户端
```

被阻止时返回给客户端的是一个**正常的 MCP 错误响应**，让 AI 知道“这个操作被安全策略拒绝了”，而不是连接崩掉。

## 4. event_type 推断

Proxy 不可能认识每个 server 的每个工具，因此用一套可配置的映射规则把工具调用归类：

- 按**工具名模式**：`*read*` → file.read，`*delete*` / `*rm*` → file.delete，`*exec*` / `*shell*` / `*command*` → shell.exec，`*query*` / `*sql*` → db.query。
- 按**参数特征**：参数里有 `path` / `file` 字段当作文件操作；有 `command` / `cmd` 当作命令；值里含 SQL 关键字当作数据库操作。
- 按 **server 类型**已知映射：内置对 filesystem、github、shell 等常见 server 的工具名做了精确映射表。

映射规则可在 `config.yaml` 里扩展，未匹配到的归为 `mcp.tool_call`（仍会评分，只是维度较粗）。

## 5. 传输模式

通过 `Transport` trait 抽象：

```rust
pub trait Transport {
    async fn recv(&mut self) -> Result<JsonRpcMessage>;
    async fn send(&mut self, msg: JsonRpcMessage) -> Result<()>;
}
```

| 模式 | 状态 | 说明 |
|---|---|---|
| stdio | MVP（P0） | 子进程方式拉起真实 server，标准输入输出对接 |
| SSE / Streamable HTTP | 规划（P1） | 面向 HTTP 类 MCP server |

新增传输只需实现 `Transport`，上层逻辑不变。

## 6. 多 Server 聚合

`config.yaml` 可注册多个真实 server。聚合模式下：

- `tools/list` 合并各 server 的工具，按 server 名加前缀避免冲突。
- `tools/call` 按前缀路由到对应 server。
- 每个 server 有独立的权限等级（见 [policy.md](policy.md) 的权限部分）。

```yaml
# config.yaml 片段
servers:
  filesystem:
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
    trust_level: 1
  github:
    command: github-mcp-server
    trust_level: 2
```

## 7. 失败与降级

- 真实 server 启动失败：`proxy start` 直接报错退出，给出明确原因，不进入半可用状态。
- 真实 server 运行中崩溃：向客户端返回错误，记录审计事件 `event_type=other, decision=error`。
- AgentShield 自身想临时摘除：客户端配置改回直连即可，无残留。

## 8. 相关需求

覆盖需求文档 7.1（MCP-001 ~ MCP-010）。详细类型与内部结构见 [docs/design/02-proxy.md](design/02-proxy.md)。
