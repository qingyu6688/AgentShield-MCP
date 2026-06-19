//! Streamable HTTP 上游传输（MCP 2025-03-26）。
//!
//! 客户端消息以 HTTP POST 发送；上游响应可能是：
//! - `202 Accepted`（无 body，多见于通知 / 响应）
//! - `application/json`（单条 JSON-RPC 消息）
//! - `text/event-stream`（SSE 流，可能含多条消息，发完即关）
//!
//! 会话 id 从 initialize 响应的 `Mcp-Session-Id` 头捕获，后续请求带回。
//!
//! 会话建立后另开一个 GET SSE 长连接，用于接收上游**主动发起**的消息
//! （sampling / elicitation 请求、进度与资源更新通知等），推回给客户端。
//!
//! 关闭说明：GET 监听线程阻塞在读取上，`close()` 只置停止标志、释放 channel
//! 发送端；该线程会在上游关闭连接或进程退出时结束，不阻塞 gateway 收尾。

use std::io::{self, BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

use crate::transport::Transport;

/// Streamable HTTP 传输。
pub struct HttpTransport {
    url: String,
    agent: ureq::Agent,
    session_id: Option<String>,
    tx: Option<Sender<String>>,
    listener_started: bool,
    running: Arc<AtomicBool>,
}

impl HttpTransport {
    pub fn new(url: &str, tx: Sender<String>) -> Self {
        HttpTransport {
            url: url.to_string(),
            agent: ureq::AgentBuilder::new().build(),
            session_id: None,
            tx: Some(tx),
            listener_started: false,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    fn push(&self, line: String) {
        if line.trim().is_empty() {
            return;
        }
        if let Some(tx) = &self.tx {
            let _ = tx.send(line);
        }
    }

    /// 开启 GET SSE 长连接监听上游主动发起的消息（只开一次）。
    fn start_listener(&mut self) {
        if self.listener_started {
            return;
        }
        self.listener_started = true;

        let tx = match &self.tx {
            Some(t) => t.clone(),
            None => return,
        };
        let url = self.url.clone();
        let agent = self.agent.clone();
        let sid = self.session_id.clone();
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut req = agent.get(&url).set("Accept", "text/event-stream");
            if let Some(s) = &sid {
                req = req.set("Mcp-Session-Id", s);
            }
            // 上游不支持 GET SSE（如返回 405）时直接放弃，不影响 POST 流程
            let resp = match req.call() {
                Ok(r) => r,
                Err(_) => return,
            };
            let ctype = resp.header("Content-Type").unwrap_or("").to_lowercase();
            if !ctype.contains("text/event-stream") {
                return;
            }
            let reader = BufReader::new(resp.into_reader());
            pump_sse(reader, Some(&running), |m| {
                let _ = tx.send(m);
            });
        });
    }
}

impl Transport for HttpTransport {
    fn send(&mut self, line: &str) -> io::Result<()> {
        let mut req = self
            .agent
            .post(&self.url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream");
        if let Some(sid) = &self.session_id {
            req = req.set("Mcp-Session-Id", sid);
        }

        let resp = match req.send_string(line) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => {
                let body = r.into_string().unwrap_or_default();
                return Err(io::Error::other(format!("上游 HTTP {code}：{body}")));
            }
            Err(e) => return Err(io::Error::other(format!("上游 HTTP 请求失败：{e}"))),
        };

        // 捕获会话 id（initialize 响应通常带回）
        if let Some(sid) = resp.header("Mcp-Session-Id") {
            self.session_id = Some(sid.to_string());
        }

        let status = resp.status();
        let ctype = resp.header("Content-Type").unwrap_or("").to_lowercase();

        if status == 202 {
            // 无 body
        } else if ctype.contains("text/event-stream") {
            let reader = BufReader::new(resp.into_reader());
            pump_sse(reader, None, |m| self.push(m));
        } else {
            // application/json 或其它：整个 body 视为一条消息
            let body = resp
                .into_string()
                .map_err(|e| io::Error::other(format!("读取上游响应失败：{e}")))?;
            self.push(body.trim().to_string());
        }

        // 首次往返后（已可能拿到会话 id）开启 GET 监听
        self.start_listener();
        Ok(())
    }

    fn close(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // 丢弃发送端；GET 监听线程的克隆会在其结束时释放
        self.tx.take();
    }
}

/// 解析 SSE 流：累积 `data:` 行，遇空行成一条消息并通过 `emit` 推出。
/// `running` 为 `Some` 时，每读到一行检查是否应停止。
fn pump_sse(reader: impl BufRead, running: Option<&AtomicBool>, mut emit: impl FnMut(String)) {
    let mut data = String::new();
    for l in reader.lines() {
        if let Some(r) = running {
            if !r.load(Ordering::Relaxed) {
                break;
            }
        }
        let l = match l {
            Ok(l) => l,
            Err(_) => break,
        };
        if l.is_empty() {
            if !data.is_empty() {
                emit(std::mem::take(&mut data));
            }
        } else if let Some(rest) = l.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        }
        // event: / id: / retry: 等字段忽略
    }
    if !data.is_empty() {
        emit(data);
    }
}
