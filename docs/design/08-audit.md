# 设计 · agentshield-audit

审计层。记录每一次工具调用、决策与确认，并据此生成报告。双写 SQLite（可查询）+ JSONL（崩溃安全、易消费）。

## 职责

- 记录安全事件、文件变更、用户确认。
- 写入前对敏感字段脱敏（调用 core 的脱敏器）。
- 支持按时间 / 风险等级 / server 查询。
- 导出 JSON / Markdown / HTML 报告。
- 异步写入，不阻塞代理主路径。

## 模块结构

```text
agentshield-audit/src/
├── lib.rs
├── sink.rs         # AuditSink trait + AuditWriter（异步入口）
├── sqlite.rs       # SQLite 存储与查询（rusqlite）
├── jsonl.rs        # JSONL 追加写
├── query.rs        # 查询条件与结果
├── report/
│   ├── mod.rs
│   ├── markdown.rs
│   ├── html.rs
│   └── json.rs
└── error.rs
```

## 表结构

```sql
CREATE TABLE security_events (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  agent_name TEXT,
  client_name TEXT,
  event_type TEXT,
  target TEXT,
  arguments_json TEXT,    -- 脱敏后
  result_json TEXT,       -- 脱敏后
  risk_score INTEGER,
  risk_level TEXT,
  decision TEXT,
  reason TEXT,
  created_at TEXT
);

CREATE TABLE file_changes (
  id TEXT PRIMARY KEY, session_id TEXT, path TEXT,
  operation TEXT, diff TEXT, risk_score INTEGER, created_at TEXT
);

CREATE TABLE approvals (
  id TEXT PRIMARY KEY, event_id TEXT,
  user_decision TEXT, remember BOOLEAN, created_at TEXT
);

CREATE TABLE mcp_servers (
  id TEXT PRIMARY KEY, name TEXT, command TEXT,
  args_json TEXT, env_json TEXT, trust_level INTEGER, enabled BOOLEAN
);

CREATE INDEX idx_events_created ON security_events(created_at);
CREATE INDEX idx_events_level   ON security_events(risk_level);
```

## 异步写入

```rust
pub struct AuditWriter {
    tx: tokio::sync::mpsc::Sender<AuditRecord>,
}

#[async_trait::async_trait]
impl AuditSink for AuditWriter {
    async fn record(&self, call: &ToolCall, decision: &Decision, result: Option<&Value>) {
        let mut rec = AuditRecord::from(call, decision, result);
        rec.redact();                 // 脱敏
        let _ = self.tx.send(rec).await;   // 只入队，不等落盘
    }
}
```

后台任务消费队列，**同时**写 JSONL（先，保证记下来）和 SQLite（后，供查询）。JSONL 用追加 + flush，进程崩溃也不丢已写记录（NF-002）。

## 查询

```rust
pub struct EventQuery {
    pub level: Option<RiskLevel>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub server: Option<String>,
    pub limit: usize,
}

pub fn query_events(db: &Connection, q: &EventQuery) -> Result<Vec<SecurityEvent>>;
```

支持按时间（AUDIT-006）、按风险等级（AUDIT-007）查询。

## 报告

`ReportBuilder` 聚合统计：

```text
项目 / 会话时间 / 客户端 / server 列表
工具调用总数、文件读写数、命令数
高危数、被阻止数、确认数
风险最高的 10 个事件
被访问的敏感文件
策略命中统计
安全建议（基于命中情况生成）
```

三种格式：

- **JSON**（REPORT-003，P0）：结构化，供二次处理。
- **Markdown**（REPORT-001）：人读，贴 PR / issue。
- **HTML**（REPORT-002）：自带样式，可分享。

可按项目或按会话生成（REPORT-007 / 008）。

## 归档

单会话支持至少 1 万条事件（NF-004）。SQLite 支持自动归档（NF-005）：超过阈值的旧事件可导出到归档文件并从主库清理，命令后续提供。

## 测试要点

- 写入→查询往返一致。
- 脱敏在落盘前生效（库里查不到明文 Token）。
- 报告统计数字正确。
- JSONL 在中途“崩溃”后已写记录仍可读。

## 相关需求

7.9 AUDIT-001 ~ AUDIT-010；7.10 REPORT-001 ~ REPORT-008；NF-002 / 004 / 005。
