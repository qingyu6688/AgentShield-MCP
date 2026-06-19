# 设计 · agentshield-proxy

MCP 代理。对客户端伪装成 MCP Server，对真实 server 充当客户端，在 `tools/call` 上插入安全决策。配套文档见 [mcp-proxy.md](../mcp-proxy.md)。

## 职责

- 实现 MCP（JSON-RPC over stdio / HTTP）的收发。
- 转发 `tools/list`，拦截 `tools/call`。
- 把原始调用解析为 `core::ToolCall`（含 `event_type` 推断与 `target` 提取）。
- 调用决策回调（risk + policy 合成），按结果放行/确认/阻止。
- 把放行结果与决策交给审计。

## 模块结构

```text
agentshield-proxy/src/
├── lib.rs
├── jsonrpc.rs      # JsonRpcMessage 编解码
├── transport.rs    # Transport trait + stdio 实现
├── transport_http.rs  # SSE/HTTP 实现（P1，规划）
├── upstream.rs     # 连接真实 server（拉子进程 / HTTP）
├── classify.rs     # event_type 推断 + target 提取
├── gateway.rs      # 拦截主循环、决策合成、转发
└── error.rs
```

## 传输抽象

```rust
#[async_trait::async_trait]
pub trait Transport: Send {
    async fn recv(&mut self) -> Result<Option<JsonRpcMessage>, ProxyError>;
    async fn send(&mut self, msg: JsonRpcMessage) -> Result<(), ProxyError>;
}
```

- `StdioTransport`：MVP。读写当前进程的 stdin/stdout（面向客户端）。
- `ChildStdioTransport`：把真实 server 作为子进程拉起，读写它的 stdin/stdout（面向上游）。
- `HttpTransport`：P1，SSE / Streamable HTTP。

新增传输只需实现 trait，gateway 逻辑不变。

## 决策回调

proxy 不直接依赖 risk / policy crate，而是接收一个回调，由 cli 注入。这样 proxy 保持纯协议职责，便于单测。

```rust
#[async_trait::async_trait]
pub trait DecisionMaker: Send + Sync {
    async fn decide(&self, call: &ToolCall) -> Decision;
}

#[async_trait::async_trait]
pub trait Approver: Send + Sync {
    /// confirm 决策时调用，返回用户裁决
    async fn approve(&self, call: &ToolCall, decision: &Decision) -> ApprovalResult;
}

#[async_trait::async_trait]
pub trait AuditSink: Send + Sync {
    async fn record(&self, call: &ToolCall, decision: &Decision, result: Option<&serde_json::Value>);
}
```

## 拦截主循环（gateway）

```text
loop:
  msg = client.recv()
  match msg.method:
    "initialize"  -> 转发，改写 serverInfo
    "tools/list"  -> upstream 取列表 → 按权限过滤/加风险提示 → 回客户端
    "tools/call"  -> handle_tool_call(msg)
    其他          -> 透传
```

`handle_tool_call`：

```text
call = classify(msg)                       // → ToolCall
decision = decision_maker.decide(&call)    // risk + policy 合成
match decision.action:
  Allow | Log:
      result = upstream.call(msg)
      audit.record(call, decision, Some(result))
      回客户端 result
  Confirm:
      match approver.approve(call, decision):
        Allow      -> 同 Allow 分支
        Deny       -> audit.record(...); 回客户端 MCP error
  Block:
      audit.record(call, decision, None)
      回客户端 MCP error（带规则名与原因）
  Sandbox:
      （P2）转交沙箱执行
```

阻止时返回标准 JSON-RPC error，`code` 用自定义区间，`message` 含原因，让 AI 能理解“被安全策略拒绝”。

## event_type 推断（classify）

按优先级：

1. **已知 server 工具映射表**：内置 filesystem / github / shell 等 server 的工具名 → EventType 精确映射。
2. **工具名模式**：`read`→FileRead，`write`/`edit`→FileWrite，`delete`/`rm`→FileDelete，`exec`/`shell`/`command`→ShellExec，`query`/`sql`→DbQuery。
3. **参数特征**：含 `path`/`file` → 文件类；含 `command`/`cmd` → ShellExec；值含 SQL 关键字 → DbQuery。
4. 兜底 `McpToolCall`。

`target` 提取：文件类取 `path` 参数，shell 取 `command`，db 取 SQL 文本。映射表可在 `config.yaml` 扩展。

## 性能

- 单条消息处理在内存完成，转发用零拷贝转发原始 bytes（仅 `tools/call` 需要解析）。
- 审计走 `tokio::sync::mpsc`，`record` 只入队不阻塞主循环（NF-002）。
- 目标：决策开销 < 100ms（NF-001），实际评分为微秒级，瓶颈在 upstream 往返。

## 测试要点

- JSON-RPC 编解码往返。
- `tools/list` 在不同权限下的过滤结果。
- 各类工具调用的 classify 正确性（用 fixtures）。
- block / confirm-deny 返回给客户端的是合法 MCP error。
- 用 mock 的 `DecisionMaker` / `Approver` / `AuditSink` 测 gateway 分支。

## 相关需求

7.1 MCP-001 ~ MCP-010。
