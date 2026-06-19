//! 拦截网关。MCP 消息穿过这里：tools/call 走决策，其余透传。
//!
//! 传输无关：上游可以是 stdio 子进程或 Streamable HTTP，统一抽象为
//! [`Transport`]（发送）+ 一条 channel（接收上游消息）。
//!
//! - 主线程：客户端 stdin → 决策 → `transport.send` 转发 / 或直接回错误给客户端
//! - 消费线程：channel（上游消息）→ 客户端 stdout（原样回传），并按 id 回填审计

use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Write};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::thread;

use agentshield_core::{ids, Action, Decision, ToolCall};

use crate::classify::classify;
use crate::http::HttpTransport;
use crate::jsonrpc::JsonRpcMessage;
use crate::transport::{StdioTransport, Transport};
use crate::{ApprovalResult, Approver, AuditSink, DecisionMaker};

/// 已转发、等待上游响应的调用：id 文本 → (调用, 决策)。
type PendingMap = HashMap<String, (ToolCall, Decision)>;

/// 一次代理会话的上下文。
pub struct ProxyContext {
    pub session_id: String,
    pub client_name: String,
    pub server_name: String,
}

impl ProxyContext {
    pub fn new(client_name: impl Into<String>, server_name: impl Into<String>) -> Self {
        ProxyContext {
            session_id: ids::new_session_id(),
            client_name: client_name.into(),
            server_name: server_name.into(),
        }
    }
}

/// 连接 stdio 上游：拉起子进程，返回传输与接收 channel。
pub fn connect_stdio(
    command: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
) -> io::Result<(Box<dyn Transport>, Receiver<String>)> {
    let (tx, rx) = mpsc::channel();
    let transport = StdioTransport::spawn(command, args, env, tx)?;
    Ok((Box::new(transport), rx))
}

/// 连接 Streamable HTTP 上游。
pub fn connect_http(url: &str) -> (Box<dyn Transport>, Receiver<String>) {
    let (tx, rx) = mpsc::channel();
    (Box::new(HttpTransport::new(url, tx)), rx)
}

/// 进入双向转发循环，直到客户端关闭。
pub fn run(
    ctx: &ProxyContext,
    dm: &dyn DecisionMaker,
    approver: &dyn Approver,
    audit: &dyn AuditSink,
    mut transport: Box<dyn Transport>,
    rx: Receiver<String>,
) -> io::Result<()> {
    // 客户端 stdout 被主线程（拦截响应）与消费线程（上游响应）共享。
    let client_out = Mutex::new(io::stdout());
    let pending: Mutex<PendingMap> = Mutex::new(HashMap::new());

    thread::scope(|s| -> io::Result<()> {
        let co = &client_out;
        let pend = &pending;
        let aud = audit;
        // 消费线程：上游消息 → 客户端 + 回填审计
        s.spawn(move || {
            for line in rx.iter() {
                if let Ok(mut w) = co.lock() {
                    let _ = w.write_all(line.as_bytes());
                    let _ = w.write_all(b"\n");
                    let _ = w.flush();
                }
                backfill(&line, pend, aud);
            }
        });

        // 主线程：客户端 → 上游，途中拦截 tools/call。
        let stdin = io::stdin();
        let mut locked = stdin.lock();
        let mut buf = String::new();
        loop {
            buf.clear();
            match locked.read_line(&mut buf) {
                Ok(0) | Err(_) => break, // 客户端关闭
                Ok(_) => {
                    let line = buf.trim_end_matches(['\n', '\r']);
                    if line.is_empty() {
                        continue;
                    }
                    let mut co = client_out
                        .lock()
                        .map_err(|_| io::Error::other("客户端输出锁中毒"))?;
                    let stop = handle_line(
                        line,
                        ctx,
                        dm,
                        approver,
                        audit,
                        &mut *co,
                        transport.as_mut(),
                        &pending,
                    )?;
                    drop(co);
                    if stop {
                        break;
                    }
                }
            }
        }

        // 关闭上游并释放 channel 发送端，消费线程随之结束（scope 会 join）
        transport.close();
        Ok(())
    })?;

    // 兜底：上游没有响应的 pending 调用，按无结果补记审计。
    if let Ok(map) = pending.into_inner() {
        for (_id, (call, decision)) in map {
            audit.record(&call, &decision, None);
        }
    }
    Ok(())
}

/// 处理客户端发来的一行消息。
///
/// 返回 `Ok(true)` 表示上游已不可用、应结束；`Ok(false)` 表示正常继续。
#[allow(clippy::too_many_arguments)]
pub fn handle_line(
    line: &str,
    ctx: &ProxyContext,
    dm: &dyn DecisionMaker,
    approver: &dyn Approver,
    audit: &dyn AuditSink,
    client_out: &mut dyn Write,
    transport: &mut dyn Transport,
    pending: &Mutex<PendingMap>,
) -> io::Result<bool> {
    let msg: JsonRpcMessage = match serde_json::from_str(line) {
        Ok(m) => m,
        // 解析不了的消息不擅自丢弃，原样透传给上游，保持协议健壮
        Err(_) => return forward(line, transport),
    };

    if !msg.is_tools_call() {
        return forward(line, transport);
    }

    // 解析 tools/call 的 name 与 arguments
    let params = msg.params.clone().unwrap_or(serde_json::Value::Null);
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let call = classify(
        &ctx.session_id,
        &ctx.client_name,
        &ctx.server_name,
        tool_name,
        arguments,
    );
    let decision = dm.decide(&call);
    let id_key = msg.id.as_ref().map(|v| v.to_string());

    match decision.action {
        // 放行：转发后等响应回填，不在此刻落审计
        Action::Allow | Action::Log | Action::Sandbox => {
            defer_or_record(pending, audit, id_key, call, decision);
            forward(line, transport)
        }
        Action::Block => {
            audit.record(&call, &decision, None);
            respond_blocked(client_out, msg.id, &decision.reason)?;
            Ok(false)
        }
        Action::Confirm => {
            let result = approver.approve(&call, &decision);
            let allowed = matches!(
                result,
                ApprovalResult::AllowOnce | ApprovalResult::AllowAlways
            );
            if allowed {
                defer_or_record(pending, audit, id_key, call, decision);
                forward(line, transport)
            } else {
                audit.record(&call, &decision, None);
                respond_blocked(client_out, msg.id, &decision.reason)?;
                Ok(false)
            }
        }
    }
}

/// 有 id 则挂入 pending 等回填；无 id 无法关联响应，直接记审计（结果为空）。
fn defer_or_record(
    pending: &Mutex<PendingMap>,
    audit: &dyn AuditSink,
    id_key: Option<String>,
    call: ToolCall,
    decision: Decision,
) {
    match id_key {
        Some(key) => {
            if let Ok(mut map) = pending.lock() {
                map.insert(key, (call, decision));
            } else {
                audit.record(&call, &decision, None);
            }
        }
        None => audit.record(&call, &decision, None),
    }
}

/// 回填：看到上游响应时，按 id 取出对应调用并连同结果写审计。
fn backfill(line: &str, pending: &Mutex<PendingMap>, audit: &dyn AuditSink) {
    let msg: JsonRpcMessage = match serde_json::from_str(line) {
        Ok(m) => m,
        Err(_) => return,
    };
    let Some(id) = &msg.id else { return };
    let key = id.to_string();
    let entry = match pending.lock() {
        Ok(mut map) => map.remove(&key),
        Err(_) => return,
    };
    if let Some((call, decision)) = entry {
        let result = extract_result(&msg);
        audit.record(&call, &decision, result.as_ref());
    }
}

/// 从上游响应中取出结果：优先 result，其次把 error 包成结果记录。
fn extract_result(msg: &JsonRpcMessage) -> Option<serde_json::Value> {
    if let Some(r) = &msg.result {
        return Some(r.clone());
    }
    if let Some(e) = &msg.error {
        return serde_json::to_value(e)
            .ok()
            .map(|ev| serde_json::json!({ "error": ev }));
    }
    None
}

/// 转发给上游。返回 `Ok(true)` 表示发送失败（上游断开）。
fn forward(line: &str, transport: &mut dyn Transport) -> io::Result<bool> {
    match transport.send(line) {
        Ok(()) => Ok(false),
        Err(e) => {
            eprintln!("[AgentShield] 转发到上游失败：{e}");
            Ok(true)
        }
    }
}

/// 向客户端回一条“被 AgentShield 阻止”的 JSON-RPC 错误响应。
fn respond_blocked(
    client_out: &mut dyn Write,
    id: Option<serde_json::Value>,
    reason: &str,
) -> io::Result<()> {
    let resp = JsonRpcMessage::blocked(id, reason);
    let s =
        serde_json::to_string(&resp).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(client_out, "{s}")?;
    client_out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::{RiskAssessment, RiskLevel};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 记录被转发到上游的消息的假传输。
    #[derive(Default)]
    struct MockTransport {
        sent: Vec<String>,
    }
    impl Transport for MockTransport {
        fn send(&mut self, line: &str) -> io::Result<()> {
            self.sent.push(line.to_string());
            Ok(())
        }
        fn close(&mut self) {}
    }

    struct FixedDecision(Action);
    impl DecisionMaker for FixedDecision {
        fn decide(&self, _call: &ToolCall) -> Decision {
            Decision {
                action: self.0,
                risk: RiskAssessment {
                    score: 50,
                    level: RiskLevel::Medium,
                    reasons: vec!["test".into()],
                    recommended_action: self.0,
                },
                matched_rule: Some("test-rule".into()),
                reason: "测试原因".into(),
            }
        }
    }

    struct AlwaysDeny;
    impl Approver for AlwaysDeny {
        fn approve(&self, _: &ToolCall, _: &Decision) -> ApprovalResult {
            ApprovalResult::Deny
        }
    }
    struct AlwaysAllow;
    impl Approver for AlwaysAllow {
        fn approve(&self, _: &ToolCall, _: &Decision) -> ApprovalResult {
            ApprovalResult::AllowOnce
        }
    }

    struct RecordingAudit {
        count: AtomicUsize,
        last_with_result: Mutex<Option<bool>>,
    }
    impl RecordingAudit {
        fn new() -> Self {
            RecordingAudit {
                count: AtomicUsize::new(0),
                last_with_result: Mutex::new(None),
            }
        }
        fn count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
        fn last_with_result(&self) -> Option<bool> {
            *self.last_with_result.lock().unwrap()
        }
    }
    impl AuditSink for RecordingAudit {
        fn record(&self, _: &ToolCall, _: &Decision, result: Option<&serde_json::Value>) {
            self.count.fetch_add(1, Ordering::SeqCst);
            *self.last_with_result.lock().unwrap() = Some(result.is_some());
        }
    }

    fn ctx() -> ProxyContext {
        ProxyContext {
            session_id: "s".into(),
            client_name: "c".into(),
            server_name: "srv".into(),
        }
    }

    const CALL: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"exec","arguments":{"command":"rm -rf /"}}}"#;

    fn pending() -> Mutex<PendingMap> {
        Mutex::new(HashMap::new())
    }

    #[test]
    fn allow_forwards_and_defers_audit() {
        let mut client_out = Vec::new();
        let mut tp = MockTransport::default();
        let audit = RecordingAudit::new();
        let pend = pending();
        handle_line(
            CALL,
            &ctx(),
            &FixedDecision(Action::Allow),
            &AlwaysDeny,
            &audit,
            &mut client_out,
            &mut tp,
            &pend,
        )
        .unwrap();
        assert_eq!(tp.sent.len(), 1);
        assert!(client_out.is_empty());
        assert_eq!(audit.count(), 0);
        assert_eq!(pend.lock().unwrap().len(), 1);
    }

    #[test]
    fn result_is_backfilled_on_response() {
        let mut client_out = Vec::new();
        let mut tp = MockTransport::default();
        let audit = RecordingAudit::new();
        let pend = pending();
        handle_line(
            CALL,
            &ctx(),
            &FixedDecision(Action::Allow),
            &AlwaysDeny,
            &audit,
            &mut client_out,
            &mut tp,
            &pend,
        )
        .unwrap();
        let resp = r#"{"jsonrpc":"2.0","id":1,"result":{"content":"ok"}}"#;
        backfill(resp, &pend, &audit);
        assert_eq!(pend.lock().unwrap().len(), 0);
        assert_eq!(audit.count(), 1);
        assert_eq!(audit.last_with_result(), Some(true));
    }

    #[test]
    fn unrelated_response_does_not_backfill() {
        let audit = RecordingAudit::new();
        let pend = pending();
        backfill(
            r#"{"jsonrpc":"2.0","id":2,"result":{"x":1}}"#,
            &pend,
            &audit,
        );
        assert_eq!(audit.count(), 0);
    }

    #[test]
    fn block_responds_error_and_does_not_forward() {
        let mut client_out = Vec::new();
        let mut tp = MockTransport::default();
        let audit = RecordingAudit::new();
        let pend = pending();
        handle_line(
            CALL,
            &ctx(),
            &FixedDecision(Action::Block),
            &AlwaysDeny,
            &audit,
            &mut client_out,
            &mut tp,
            &pend,
        )
        .unwrap();
        assert!(tp.sent.is_empty());
        let out = String::from_utf8(client_out).unwrap();
        assert!(out.contains("blocked by AgentShield"));
        assert_eq!(audit.count(), 1);
        assert!(pend.lock().unwrap().is_empty());
    }

    #[test]
    fn confirm_denied_blocks() {
        let mut client_out = Vec::new();
        let mut tp = MockTransport::default();
        let audit = RecordingAudit::new();
        let pend = pending();
        handle_line(
            CALL,
            &ctx(),
            &FixedDecision(Action::Confirm),
            &AlwaysDeny,
            &audit,
            &mut client_out,
            &mut tp,
            &pend,
        )
        .unwrap();
        assert!(tp.sent.is_empty());
        assert!(String::from_utf8(client_out).unwrap().contains("blocked"));
        assert_eq!(audit.count(), 1);
    }

    #[test]
    fn confirm_allowed_forwards_and_defers() {
        let mut client_out = Vec::new();
        let mut tp = MockTransport::default();
        let audit = RecordingAudit::new();
        let pend = pending();
        handle_line(
            CALL,
            &ctx(),
            &FixedDecision(Action::Confirm),
            &AlwaysAllow,
            &audit,
            &mut client_out,
            &mut tp,
            &pend,
        )
        .unwrap();
        assert_eq!(tp.sent.len(), 1);
        assert!(client_out.is_empty());
        assert_eq!(audit.count(), 0);
        assert_eq!(pend.lock().unwrap().len(), 1);
    }

    #[test]
    fn non_tools_call_is_forwarded() {
        let mut client_out = Vec::new();
        let mut tp = MockTransport::default();
        let audit = RecordingAudit::new();
        let pend = pending();
        handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            &ctx(),
            &FixedDecision(Action::Block),
            &AlwaysDeny,
            &audit,
            &mut client_out,
            &mut tp,
            &pend,
        )
        .unwrap();
        assert_eq!(tp.sent.len(), 1);
        assert!(client_out.is_empty());
        assert_eq!(audit.count(), 0);
        assert!(pend.lock().unwrap().is_empty());
    }
}
