//! Streamable HTTP 上游传输（MCP 2025-03-26）。
//!
//! 客户端消息以 HTTP POST 发送；上游响应可能是：
//! - `202 Accepted`（无 body，多见于通知 / 响应）
//! - `application/json`（单条 JSON-RPC 消息）
//! - `text/event-stream`（SSE 流，可能含多条消息，发完即关）
//!
//! 会话 id 从 initialize 响应的 `Mcp-Session-Id` 头捕获，后续请求带回。
//!
//! 限制（MVP）：仅通过 POST 响应接收上游消息，未单独维持 GET SSE 长连接，
//! 因此上游主动发起的请求（如 sampling）暂不支持，见 docs/design/02-proxy.md。

use std::io::{self, BufRead, BufReader};
use std::sync::mpsc::Sender;

use crate::transport::Transport;

/// Streamable HTTP 传输。
pub struct HttpTransport {
    url: String,
    agent: ureq::Agent,
    session_id: Option<String>,
    tx: Option<Sender<String>>,
}

impl HttpTransport {
    pub fn new(url: &str, tx: Sender<String>) -> Self {
        HttpTransport {
            url: url.to_string(),
            agent: ureq::AgentBuilder::new().build(),
            session_id: None,
            tx: Some(tx),
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
            return Ok(()); // 无 body
        }

        if ctype.contains("text/event-stream") {
            // 解析 SSE：累积 data: 行，遇空行成一条消息，边解析边推送
            let reader = BufReader::new(resp.into_reader());
            let mut data = String::new();
            for l in reader.lines() {
                let l = match l {
                    Ok(l) => l,
                    Err(_) => break,
                };
                if l.is_empty() {
                    if !data.is_empty() {
                        self.push(std::mem::take(&mut data));
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
                self.push(data);
            }
        } else {
            // application/json 或其它：整个 body 视为一条消息
            let body = resp
                .into_string()
                .map_err(|e| io::Error::other(format!("读取上游响应失败：{e}")))?;
            self.push(body.trim().to_string());
        }

        Ok(())
    }

    fn close(&mut self) {
        // 丢弃发送端，让 gateway 的消费线程在 channel 关闭后结束
        self.tx.take();
    }
}
