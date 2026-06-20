//! 多 MCP Server 聚合。
//!
//! 一个 AgentShield 入口连接多个上游：
//! - 启动时替客户端完成各上游的 initialize 握手（prime），客户端的 initialize 直接合成回复；
//! - `tools/list` 扇出到所有上游并合并，工具名加 `server__` 前缀避免冲突；
//! - `tools/call` 按前缀路由到对应上游，仍走完整决策与审计；
//! - 通知广播到所有上游，其它请求路由到第一个上游（MVP 限制）。
//!
//! 线程模型与单上游一致：主线程处理客户端→上游，消费线程处理上游→客户端
//! （含 tools/list 扇出结果合并与 tools/call 审计回填）。

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use agentshield_core::{ids, Action};
use serde_json::{json, Value};

use crate::classify::classify;
use crate::gateway::{backfill, respond_blocked, PendingMap};
use crate::transport::Transport;
use crate::{ApprovalResult, Approver, AuditSink, DecisionMaker};

/// 工具名前缀分隔符：`<server>__<tool>`。
const SEP: &str = "__";

/// 一个上游连接（聚合输入）。
pub struct UpstreamConn {
    pub name: String,
    pub transport: Box<dyn Transport>,
    pub rx: Receiver<String>,
}

/// 一次 tools/list 扇出的收集状态。
struct Fanout {
    client_id: Value,
    remaining: usize,
    tools: Vec<Value>,
}

/// 启动聚合代理。
pub fn run_aggregate(
    client_name: &str,
    dm: &dyn DecisionMaker,
    approver: &dyn Approver,
    audit: &dyn AuditSink,
    mut conns: Vec<UpstreamConn>,
) -> io::Result<()> {
    if conns.is_empty() {
        return Err(io::Error::other("聚合模式至少需要一个上游 server"));
    }

    // 1. 替客户端先把每个上游 initialize 握手做掉
    for c in &mut conns {
        prime_upstream(c);
    }

    let session_id = ids::new_session_id();
    let names: Vec<String> = conns.iter().map(|c| c.name.clone()).collect();

    // 2. 拆分：transports 给主线程路由，rx 移入各自的 tagger 线程
    let mut transports: Vec<Box<dyn Transport>> = Vec::with_capacity(conns.len());
    let mut rxs: Vec<Receiver<String>> = Vec::with_capacity(conns.len());
    for c in conns {
        transports.push(c.transport);
        rxs.push(c.rx);
    }

    // 合并 channel：(上游下标, 消息行)
    let (merged_tx, merged_rx) = mpsc::channel::<(usize, String)>();

    let client_out = Mutex::new(io::stdout());
    let pending: Mutex<PendingMap> = Mutex::new(HashMap::new());
    let fanout: Mutex<HashMap<u64, Fanout>> = Mutex::new(HashMap::new());
    let seq = AtomicU64::new(0);
    let stop = AtomicBool::new(false);

    // tagger 线程：把每个上游的 rx 打上下标转入合并 channel（独立 detached 线程）
    for (idx, rx) in rxs.into_iter().enumerate() {
        let tx = merged_tx.clone();
        thread::spawn(move || {
            for line in rx.iter() {
                if tx.send((idx, line)).is_err() {
                    break;
                }
            }
        });
    }
    drop(merged_tx); // 仅 tagger 持有发送端

    thread::scope(|s| -> io::Result<()> {
        let co = &client_out;
        let pend = &pending;
        let fo = &fanout;
        let nm = &names;
        let aud = audit;
        let stop_ref = &stop;

        // 消费线程：上游 → 客户端
        s.spawn(move || {
            loop {
                match merged_rx.recv_timeout(Duration::from_millis(150)) {
                    Ok((idx, line)) => handle_upstream(idx, &line, nm, co, pend, fo, aud),
                    Err(RecvTimeoutError::Timeout) => {
                        if stop_ref.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            while let Ok((idx, line)) = merged_rx.try_recv() {
                handle_upstream(idx, &line, nm, co, pend, fo, aud);
            }
        });

        // 主线程：客户端 → 上游
        let stdin = io::stdin();
        let mut locked = stdin.lock();
        let mut buf = String::new();
        loop {
            buf.clear();
            match locked.read_line(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = buf.trim_end_matches(['\n', '\r']);
                    if line.is_empty() {
                        continue;
                    }
                    handle_client(
                        line,
                        &session_id,
                        client_name,
                        dm,
                        approver,
                        audit,
                        &names,
                        &mut transports,
                        &client_out,
                        &pending,
                        &fanout,
                        &seq,
                    )?;
                }
            }
        }

        for t in transports.iter_mut() {
            t.close();
        }
        stop.store(true, Ordering::Relaxed);
        Ok(())
    })?;

    // 兜底：剩余未回填的调用补记审计
    if let Ok(map) = pending.into_inner() {
        for (_id, (call, decision)) in map {
            audit.record(&call, &decision, None);
        }
    }
    Ok(())
}

/// 替客户端把一个上游的 initialize 握手做掉（响应丢弃）。
fn prime_upstream(c: &mut UpstreamConn) {
    let init = json!({
        "jsonrpc": "2.0",
        "id": "agshield-init",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "AgentShield", "version": "0.1.0" }
        }
    });
    if c.transport.send(&init.to_string()).is_err() {
        return;
    }
    // 丢弃 initialize 响应（http 会在 send 时把响应推入 rx；stdio 异步到达）
    let _ = c.rx.recv_timeout(Duration::from_secs(5));
    let _ = c
        .transport
        .send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string());
}

/// 主线程：处理客户端一行消息。
#[allow(clippy::too_many_arguments)]
fn handle_client(
    line: &str,
    session_id: &str,
    client_name: &str,
    dm: &dyn DecisionMaker,
    approver: &dyn Approver,
    audit: &dyn AuditSink,
    names: &[String],
    transports: &mut [Box<dyn Transport>],
    client_out: &Mutex<io::Stdout>,
    pending: &Mutex<PendingMap>,
    fanout: &Mutex<HashMap<u64, Fanout>>,
    seq: &AtomicU64,
) -> io::Result<()> {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Ok(()), // 无法解析，忽略
    };
    let method = v.get("method").and_then(|m| m.as_str());
    let has_id = v.get("id").map(|i| !i.is_null()).unwrap_or(false);

    match method {
        Some("initialize") => {
            // 直接合成回复，上游已在启动时 prime
            let resp = json!({
                "jsonrpc": "2.0",
                "id": v.get("id").cloned().unwrap_or(Value::Null),
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "AgentShield", "version": "0.1.0" }
                }
            });
            write_client(client_out, &resp.to_string());
            Ok(())
        }
        Some("notifications/initialized") => Ok(()), // 已在 prime 阶段处理，吞掉
        Some("tools/list") => {
            let id = v.get("id").cloned().unwrap_or(Value::Null);
            let s = seq.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut fo) = fanout.lock() {
                fo.insert(
                    s,
                    Fanout {
                        client_id: id,
                        remaining: transports.len(),
                        tools: Vec::new(),
                    },
                );
            }
            for (idx, t) in transports.iter_mut().enumerate() {
                let req = json!({"jsonrpc":"2.0","id": format!("agshield:{s}:{idx}"), "method":"tools/list"});
                let _ = t.send(&req.to_string());
            }
            Ok(())
        }
        Some("tools/call") => handle_tools_call(
            &v,
            session_id,
            client_name,
            dm,
            approver,
            audit,
            names,
            transports,
            client_out,
            pending,
        ),
        Some(_) if !has_id => {
            // 通知：广播到所有上游
            for t in transports.iter_mut() {
                let _ = t.send(line);
            }
            Ok(())
        }
        _ => {
            // 其它请求：路由到第一个上游（MVP 限制）
            if let Some(t) = transports.first_mut() {
                let _ = t.send(line);
            }
            Ok(())
        }
    }
}

/// 路由并拦截一次 tools/call。
#[allow(clippy::too_many_arguments)]
fn handle_tools_call(
    v: &Value,
    session_id: &str,
    client_name: &str,
    dm: &dyn DecisionMaker,
    approver: &dyn Approver,
    audit: &dyn AuditSink,
    names: &[String],
    transports: &mut [Box<dyn Transport>],
    client_out: &Mutex<io::Stdout>,
    pending: &Mutex<PendingMap>,
) -> io::Result<()> {
    let client_id = v.get("id").cloned();
    let prefixed = v
        .get("params")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("");

    // 拆分 server__tool 前缀
    let Some((server, tool)) = prefixed.split_once(SEP) else {
        return reply_error(
            client_out,
            client_id,
            &format!("工具名缺少 server 前缀（应为 server{SEP}tool）：{prefixed}"),
        );
    };
    let Some(idx) = names.iter().position(|n| n == server) else {
        return reply_error(
            client_out,
            client_id,
            &format!("未知的 server 前缀：{server}"),
        );
    };

    // 还原成上游真实工具名后再转发
    let mut fwd = v.clone();
    fwd["params"]["name"] = Value::String(tool.to_string());
    let arguments = fwd
        .get("params")
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(Value::Null);

    let call = classify(session_id, client_name, server, tool, arguments);
    let decision = dm.decide(&call);
    let id_key = client_id.as_ref().map(|i| i.to_string());

    let forward = |transports: &mut [Box<dyn Transport>]| {
        let _ = transports[idx].send(&fwd.to_string());
    };

    match decision.action {
        Action::Allow | Action::Log | Action::Sandbox => {
            if let (Some(key), Ok(mut p)) = (id_key, pending.lock()) {
                p.insert(key, (call, decision));
            } else {
                audit.record(&call, &decision, None);
            }
            forward(transports);
            Ok(())
        }
        Action::Block => {
            audit.record(&call, &decision, None);
            respond_blocked(&mut *lock_out(client_out), client_id, &decision.reason)
        }
        Action::Confirm => {
            let result = approver.approve(&call, &decision);
            let allowed = matches!(
                result,
                ApprovalResult::AllowOnce | ApprovalResult::AllowAlways
            );
            if allowed {
                if let (Some(key), Ok(mut p)) = (id_key, pending.lock()) {
                    p.insert(key, (call, decision));
                } else {
                    audit.record(&call, &decision, None);
                }
                forward(transports);
                Ok(())
            } else {
                audit.record(&call, &decision, None);
                respond_blocked(&mut *lock_out(client_out), client_id, &decision.reason)
            }
        }
    }
}

/// 消费线程：处理一条上游消息。
fn handle_upstream(
    idx: usize,
    line: &str,
    names: &[String],
    client_out: &Mutex<io::Stdout>,
    pending: &Mutex<PendingMap>,
    fanout: &Mutex<HashMap<u64, Fanout>>,
    audit: &dyn AuditSink,
) {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            write_client(client_out, line); // 无法解析，原样透传
            return;
        }
    };

    // 是否为 tools/list 扇出响应？id 形如 "agshield:<seq>:<idx>"
    if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
        if let Some((seq, _idx)) = parse_fanout_id(id) {
            handle_fanout_response(seq, idx, &v, names, client_out, fanout);
            return;
        }
    }

    // 普通响应 / 通知：透传客户端，并对 tools/call 结果回填审计
    write_client(client_out, line);
    if v.get("id").is_some() {
        backfill(line, pending, audit);
    }
}

/// 合并 tools/list 扇出响应，集齐后回客户端。
fn handle_fanout_response(
    seq: u64,
    idx: usize,
    v: &Value,
    names: &[String],
    client_out: &Mutex<io::Stdout>,
    fanout: &Mutex<HashMap<u64, Fanout>>,
) {
    let mut fo = match fanout.lock() {
        Ok(f) => f,
        Err(_) => return,
    };
    let Some(entry) = fo.get_mut(&seq) else {
        return;
    };

    if let Some(tools) = v
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
    {
        let prefix = names.get(idx).cloned().unwrap_or_default();
        for tool in tools {
            let mut t = tool.clone();
            if let Some(name) = t.get("name").and_then(|n| n.as_str()) {
                t["name"] = Value::String(format!("{prefix}{SEP}{name}"));
            }
            entry.tools.push(t);
        }
    }

    entry.remaining = entry.remaining.saturating_sub(1);
    if entry.remaining == 0 {
        let done = fo.remove(&seq).unwrap();
        let resp = json!({
            "jsonrpc": "2.0",
            "id": done.client_id,
            "result": { "tools": done.tools }
        });
        write_client(client_out, &resp.to_string());
    }
}

fn parse_fanout_id(id: &str) -> Option<(u64, usize)> {
    let rest = id.strip_prefix("agshield:")?;
    let (seq, idx) = rest.split_once(':')?;
    Some((seq.parse().ok()?, idx.parse().ok()?))
}

fn reply_error(client_out: &Mutex<io::Stdout>, id: Option<Value>, msg: &str) -> io::Result<()> {
    respond_blocked(&mut *lock_out(client_out), id, msg)
}

fn write_client(client_out: &Mutex<io::Stdout>, line: &str) {
    let mut w = lock_out(client_out);
    let _ = w.write_all(line.as_bytes());
    let _ = w.write_all(b"\n");
    let _ = w.flush();
}

fn lock_out(client_out: &Mutex<io::Stdout>) -> std::sync::MutexGuard<'_, io::Stdout> {
    client_out.lock().unwrap_or_else(|e| e.into_inner())
}
