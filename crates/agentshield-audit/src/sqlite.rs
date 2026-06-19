//! SQLite 审计存储。用于结构化查询与报告，表结构见 docs/design/08-audit.md。
//!
//! `Connection` 自身非 `Sync`，用 `Mutex` 包起来，让 `SqliteStore` 满足审计接口
//! 所需的 `Send + Sync`。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, params_from_iter, Connection};

use crate::error::Result;
use crate::AuditRecord;

/// 审计事件查询条件。各字段是 AND 关系，None 表示不限。
#[derive(Debug, Default, Clone)]
pub struct EventQuery {
    /// 按风险等级过滤（low / medium / high / critical），大小写不敏感
    pub level: Option<String>,
    /// 按来源 MCP server 名过滤
    pub server: Option<String>,
    /// 起始时间下界（含），RFC3339 文本
    pub since: Option<String>,
    /// 结束时间上界（含），RFC3339 文本
    pub until: Option<String>,
    /// 最多返回多少条；None 表示不限
    pub limit: Option<usize>,
}

/// SQLite 审计存储。
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// 打开（或创建）数据库并建表。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = SqliteStore {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS security_events (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              agent_name TEXT,
              client_name TEXT,
              server_name TEXT,
              event_type TEXT,
              tool_name TEXT,
              target TEXT,
              arguments_json TEXT,
              result_json TEXT,
              risk_score INTEGER,
              risk_level TEXT,
              decision TEXT,
              reason TEXT,
              created_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_events_created ON security_events(created_at);
            CREATE INDEX IF NOT EXISTS idx_events_level   ON security_events(risk_level);

            CREATE TABLE IF NOT EXISTS approvals (
              id TEXT PRIMARY KEY,
              event_id TEXT,
              user_decision TEXT,
              remember INTEGER,
              created_at TEXT
            );

            CREATE TABLE IF NOT EXISTS mcp_servers (
              id TEXT PRIMARY KEY,
              name TEXT,
              command TEXT,
              args_json TEXT,
              env_json TEXT,
              trust_level INTEGER,
              enabled INTEGER
            );
            "#,
        )?;
        Ok(())
    }

    /// 写入一条审计记录（记录在 [`AuditRecord::build`] 阶段已脱敏）。
    pub fn insert(&self, rec: &AuditRecord) -> Result<()> {
        let args = serde_json::to_string(&rec.arguments_json)?;
        let result = match &rec.result_json {
            Some(v) => Some(serde_json::to_string(v)?),
            None => None,
        };
        let conn = self.lock();
        conn.execute(
            r#"INSERT OR REPLACE INTO security_events
               (id, session_id, agent_name, client_name, server_name, event_type,
                tool_name, target, arguments_json, result_json, risk_score,
                risk_level, decision, reason, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)"#,
            params![
                rec.id,
                rec.session_id,
                rec.client_name, // agent_name 暂与 client 一致
                rec.client_name,
                rec.server_name,
                rec.event_type,
                rec.tool_name,
                rec.target,
                args,
                result,
                rec.risk_score,
                rec.risk_level,
                rec.decision,
                rec.reason,
                rec.created_at,
            ],
        )?;
        Ok(())
    }

    /// 按条件查询事件，按时间倒序（最新在前）。
    pub fn query(&self, q: &EventQuery) -> Result<Vec<AuditRecord>> {
        // 动态拼接 WHERE 条件，参数按出现顺序绑定
        let mut conds: Vec<&str> = Vec::new();
        let mut vals: Vec<String> = Vec::new();
        if let Some(l) = &q.level {
            conds.push("risk_level = ? COLLATE NOCASE");
            vals.push(l.clone());
        }
        if let Some(s) = &q.server {
            conds.push("server_name = ?");
            vals.push(s.clone());
        }
        if let Some(s) = &q.since {
            conds.push("created_at >= ?");
            vals.push(s.clone());
        }
        if let Some(u) = &q.until {
            conds.push("created_at <= ?");
            vals.push(u.clone());
        }

        let mut sql = String::from(
            r#"SELECT id, session_id, client_name, server_name, event_type, tool_name,
                      target, arguments_json, result_json, risk_score, risk_level,
                      decision, reason, created_at
               FROM security_events"#,
        );
        if !conds.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conds.join(" AND "));
        }
        sql.push_str(" ORDER BY created_at DESC");
        if let Some(n) = q.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }

        let conn = self.lock();
        let mut stmt = conn.prepare(&sql)?;
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<AuditRecord> {
            let args: String = row.get(7)?;
            let result: Option<String> = row.get(8)?;
            Ok(AuditRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                client_name: row.get(2)?,
                server_name: row.get(3)?,
                event_type: row.get(4)?,
                tool_name: row.get(5)?,
                target: row.get(6)?,
                arguments_json: serde_json::from_str(&args).unwrap_or(serde_json::Value::Null),
                result_json: result.and_then(|s| serde_json::from_str(&s).ok()),
                risk_score: row.get(9)?,
                risk_level: row.get(10)?,
                decision: row.get(11)?,
                reason: row.get(12)?,
                created_at: row.get(13)?,
            })
        };

        let rows = stmt
            .query_map(params_from_iter(vals.iter()), map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        // 锁中毒时仍取回内部连接，避免审计因一次 panic 永久不可用
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::{Action, Decision, EventType, RiskAssessment, RiskLevel, ToolCall};
    use chrono::Utc;

    fn rec(target: &str, level: RiskLevel, action: Action) -> AuditRecord {
        let call = ToolCall {
            id: uuid_like(target),
            session_id: "s".into(),
            client_name: "c".into(),
            server_name: "srv".into(),
            tool_name: "read".into(),
            event_type: EventType::FileRead,
            target: Some(target.into()),
            arguments: serde_json::json!({"path": target}),
            created_at: Utc::now(),
        };
        let decision = Decision {
            action,
            risk: RiskAssessment {
                score: 80,
                level,
                reasons: vec!["r".into()],
                recommended_action: action,
            },
            matched_rule: None,
            reason: "测试".into(),
        };
        AuditRecord::build(&call, &decision, Some(&serde_json::json!({"content": "x"})))
    }

    fn uuid_like(s: &str) -> String {
        format!("id-{s}")
    }

    #[test]
    fn insert_and_query_roundtrip() {
        let store = SqliteStore::open(":memory:").unwrap();
        store
            .insert(&rec("./a.txt", RiskLevel::Low, Action::Log))
            .unwrap();
        store
            .insert(&rec("./.env", RiskLevel::Critical, Action::Block))
            .unwrap();

        let all = store.query(&EventQuery::default()).unwrap();
        assert_eq!(all.len(), 2);

        let crit = store
            .query(&EventQuery {
                level: Some("critical".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(crit.len(), 1);
        assert_eq!(crit[0].target.as_deref(), Some("./.env"));
        // 结果字段被持久化并取回
        assert!(crit[0].result_json.is_some());
    }

    #[test]
    fn limit_is_applied() {
        let store = SqliteStore::open(":memory:").unwrap();
        for i in 0..5 {
            store
                .insert(&rec(&format!("./f{i}.txt"), RiskLevel::Low, Action::Log))
                .unwrap();
        }
        let some = store
            .query(&EventQuery {
                limit: Some(3),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(some.len(), 3);
    }

    #[test]
    fn filters_by_server_and_time_range() {
        let store = SqliteStore::open(":memory:").unwrap();
        // rec() 里 server_name 固定为 "srv"，created_at 为当前时间
        store
            .insert(&rec("./.env", RiskLevel::Critical, Action::Block))
            .unwrap();

        let by_server = |s: &str| {
            store
                .query(&EventQuery {
                    server: Some(s.into()),
                    ..Default::default()
                })
                .unwrap()
                .len()
        };
        assert_eq!(by_server("srv"), 1);
        assert_eq!(by_server("nope"), 0);

        let since = |t: &str| {
            store
                .query(&EventQuery {
                    since: Some(t.into()),
                    ..Default::default()
                })
                .unwrap()
                .len()
        };
        assert_eq!(since("2000-01-01T00:00:00+00:00"), 1); // 过去下界，包含
        assert_eq!(since("2999-01-01T00:00:00+00:00"), 0); // 未来下界，排除

        let until_past = store
            .query(&EventQuery {
                until: Some("2000-01-01T00:00:00+00:00".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(until_past.len(), 0); // 过去上界，排除
    }
}
