//! 审计报告生成。从一批 [`AuditRecord`] 汇总统计，输出 JSON / Markdown / HTML。

use serde::Serialize;

use crate::error::Result;
use crate::AuditRecord;

/// 报告格式。
#[derive(Debug, Clone, Copy)]
pub enum Format {
    Json,
    Markdown,
    Html,
}

impl Format {
    /// 从字符串解析，无法识别返回 None。
    pub fn parse(s: &str) -> Option<Format> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Some(Format::Json),
            "markdown" | "md" => Some(Format::Markdown),
            "html" => Some(Format::Html),
            _ => None,
        }
    }
}

/// 报告元信息。
#[derive(Debug, Clone, Serialize)]
pub struct ReportMeta {
    pub project: String,
    pub generated_at: String,
}

/// 汇总统计。
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total: usize,
    pub file_reads: usize,
    pub file_writes: usize,
    pub file_deletes: usize,
    pub shell_commands: usize,
    pub db_queries: usize,
    pub blocked: usize,
    pub confirmed: usize,
    pub high_risk: usize,
}

/// 单条精简事件，用于排行与列表。
#[derive(Debug, Clone, Serialize)]
pub struct EventBrief {
    pub risk_level: String,
    pub risk_score: u8,
    pub event_type: String,
    pub target: String,
    pub decision: String,
}

/// 完整报告。
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub meta: ReportMeta,
    pub summary: Summary,
    pub top_events: Vec<EventBrief>,
    pub blocked_targets: Vec<String>,
    pub recommendations: Vec<String>,
}

impl Report {
    /// 从记录与元信息汇总出报告。
    pub fn build(records: &[AuditRecord], meta: ReportMeta) -> Report {
        let count = |et: &str| records.iter().filter(|r| r.event_type == et).count();

        let summary = Summary {
            total: records.len(),
            file_reads: count("FileRead"),
            file_writes: count("FileWrite") + count("FileRename"),
            file_deletes: count("FileDelete"),
            shell_commands: count("ShellExec"),
            db_queries: count("DbQuery"),
            blocked: records.iter().filter(|r| r.decision == "Block").count(),
            confirmed: records.iter().filter(|r| r.decision == "Confirm").count(),
            high_risk: records
                .iter()
                .filter(|r| r.risk_level == "High" || r.risk_level == "Critical")
                .count(),
        };

        // 风险最高的前 10 条
        let mut sorted: Vec<&AuditRecord> = records.iter().collect();
        sorted.sort_by_key(|r| std::cmp::Reverse(r.risk_score));
        let top_events = sorted
            .iter()
            .take(10)
            .map(|r| EventBrief {
                risk_level: r.risk_level.clone(),
                risk_score: r.risk_score,
                event_type: r.event_type.clone(),
                target: r.target.clone().unwrap_or_else(|| "-".into()),
                decision: r.decision.clone(),
            })
            .collect();

        // 被阻止的目标（去重）
        let mut blocked_targets: Vec<String> = records
            .iter()
            .filter(|r| r.decision == "Block")
            .filter_map(|r| r.target.clone())
            .collect();
        blocked_targets.sort();
        blocked_targets.dedup();

        let recommendations = build_recommendations(&summary, &blocked_targets);

        Report {
            meta,
            summary,
            top_events,
            blocked_targets,
            recommendations,
        }
    }

    /// 按指定格式渲染。
    pub fn render(&self, fmt: Format) -> Result<String> {
        Ok(match fmt {
            Format::Json => serde_json::to_string_pretty(self)?,
            Format::Markdown => self.render_markdown(),
            Format::Html => self.render_html(),
        })
    }

    fn render_markdown(&self) -> String {
        let s = &self.summary;
        let mut out = String::new();
        out.push_str("# AgentShield 审计报告\n\n");
        out.push_str(&format!("项目：{}\n", self.meta.project));
        out.push_str(&format!("生成时间：{}\n\n", self.meta.generated_at));

        out.push_str("## 概要\n\n");
        out.push_str(&format!("- 工具调用总数：{}\n", s.total));
        out.push_str(&format!(
            "- 文件读取：{}    文件写入：{}    文件删除：{}\n",
            s.file_reads, s.file_writes, s.file_deletes
        ));
        out.push_str(&format!(
            "- Shell 命令：{}    数据库操作：{}\n",
            s.shell_commands, s.db_queries
        ));
        out.push_str(&format!(
            "- 被阻止：{}    需确认：{}    高危：{}\n\n",
            s.blocked, s.confirmed, s.high_risk
        ));

        if !self.top_events.is_empty() {
            out.push_str("## 风险最高的事件\n\n");
            for (i, e) in self.top_events.iter().enumerate() {
                out.push_str(&format!(
                    "{}. [{}/{}] {} {} → {}\n",
                    i + 1,
                    e.risk_level,
                    e.risk_score,
                    e.event_type,
                    e.target,
                    e.decision
                ));
            }
            out.push('\n');
        }

        if !self.blocked_targets.is_empty() {
            out.push_str("## 被阻止的目标\n\n");
            for t in &self.blocked_targets {
                out.push_str(&format!("- {t}\n"));
            }
            out.push('\n');
        }

        if !self.recommendations.is_empty() {
            out.push_str("## 安全建议\n\n");
            for r in &self.recommendations {
                out.push_str(&format!("- {r}\n"));
            }
            out.push('\n');
        }
        out
    }

    fn render_html(&self) -> String {
        let body = html_escape(&self.render_markdown());
        format!(
            r#"<!doctype html>
<html lang="zh">
<head>
<meta charset="utf-8">
<title>AgentShield 审计报告</title>
<style>
  body {{ font-family: -apple-system, "Segoe UI", system-ui, sans-serif; max-width: 820px;
          margin: 40px auto; padding: 0 20px; color: #1f2329; line-height: 1.6; }}
  pre {{ white-space: pre-wrap; word-break: break-word; }}
</style>
</head>
<body>
<pre>{body}</pre>
</body>
</html>
"#
        )
    }
}

fn build_recommendations(summary: &Summary, blocked_targets: &[String]) -> Vec<String> {
    let mut recs = Vec::new();
    if summary.blocked > 0 {
        recs.push(format!(
            "本次有 {} 次操作被阻止，确认这些是否为预期的 AI 行为。",
            summary.blocked
        ));
    }
    if blocked_targets.iter().any(|t| t.contains(".env")) {
        recs.push("检测到对 .env 的访问尝试，确认 AI 工作流是否真的需要读取密钥文件。".into());
    }
    if summary.high_risk > 0 {
        recs.push(format!(
            "存在 {} 次高危及以上操作，建议复核策略是否足够严格。",
            summary.high_risk
        ));
    }
    if recs.is_empty() {
        recs.push("未发现明显异常。".into());
    }
    recs
}

/// 极简 HTML 转义，避免报告内容破坏页面结构。
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::{Action, Decision, EventType, RiskAssessment, RiskLevel, ToolCall};
    use chrono::Utc;

    fn rec(
        et: EventType,
        target: &str,
        level: RiskLevel,
        action: Action,
        score: u8,
    ) -> AuditRecord {
        let call = ToolCall {
            id: format!("id-{target}"),
            session_id: "s".into(),
            client_name: "c".into(),
            server_name: "srv".into(),
            tool_name: "t".into(),
            event_type: et,
            target: Some(target.into()),
            arguments: serde_json::Value::Null,
            created_at: Utc::now(),
        };
        let decision = Decision {
            action,
            risk: RiskAssessment {
                score,
                level,
                reasons: vec![],
                recommended_action: action,
            },
            matched_rule: None,
            reason: "r".into(),
        };
        AuditRecord::build(&call, &decision, None)
    }

    fn sample() -> Vec<AuditRecord> {
        vec![
            rec(
                EventType::FileRead,
                "./src/a.rs",
                RiskLevel::Low,
                Action::Log,
                10,
            ),
            rec(
                EventType::FileRead,
                "./.env",
                RiskLevel::Critical,
                Action::Block,
                95,
            ),
            rec(
                EventType::ShellExec,
                "rm -rf x",
                RiskLevel::High,
                Action::Confirm,
                60,
            ),
        ]
    }

    fn meta() -> ReportMeta {
        ReportMeta {
            project: "demo".into(),
            generated_at: "2026-06-19".into(),
        }
    }

    #[test]
    fn summary_counts_correct() {
        let r = Report::build(&sample(), meta());
        assert_eq!(r.summary.total, 3);
        assert_eq!(r.summary.file_reads, 2);
        assert_eq!(r.summary.shell_commands, 1);
        assert_eq!(r.summary.blocked, 1);
        assert_eq!(r.summary.confirmed, 1);
        assert_eq!(r.summary.high_risk, 2); // High + Critical
    }

    #[test]
    fn top_events_sorted_desc() {
        let r = Report::build(&sample(), meta());
        assert_eq!(r.top_events[0].risk_score, 95);
        assert_eq!(r.top_events.last().unwrap().risk_score, 10);
    }

    #[test]
    fn blocked_targets_and_env_recommendation() {
        let r = Report::build(&sample(), meta());
        assert_eq!(r.blocked_targets, vec!["./.env".to_string()]);
        assert!(r.recommendations.iter().any(|x| x.contains(".env")));
    }

    #[test]
    fn renders_all_formats() {
        let r = Report::build(&sample(), meta());
        assert!(r.render(Format::Markdown).unwrap().contains("审计报告"));
        assert!(r.render(Format::Json).unwrap().contains("\"total\""));
        assert!(r.render(Format::Html).unwrap().contains("<html"));
    }
}
