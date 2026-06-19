//! 上游传输抽象。
//!
//! 对客户端侧 AgentShield 始终是 stdio MCP server；对真实上游可以是：
//! - stdio：把上游作为子进程拉起（[`StdioTransport`]）
//! - Streamable HTTP：POST 发消息、响应可能是 JSON 或 SSE 流（见 `http` 模块）
//!
//! 两种传输统一通过一个 [`Sender`] 把「上游 → 客户端」的消息推回来，
//! 由 gateway 的消费线程写给客户端并回填审计。

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

/// 上游传输：负责把客户端消息发给真实 server。
/// 上游回来的消息由各实现推入构造时给定的 channel。
pub trait Transport: Send {
    /// 发送一条客户端 → 上游的 JSON-RPC 消息（单行，无换行）。
    fn send(&mut self, line: &str) -> io::Result<()>;
    /// 收尾：关闭上游、释放 channel 发送端，让消费线程结束。
    fn close(&mut self);
}

/// stdio 传输：上游作为子进程，对接它的 stdin/stdout。
pub struct StdioTransport {
    child: Child,
    stdin: Option<ChildStdin>,
    reader: Option<JoinHandle<()>>,
}

impl StdioTransport {
    /// 拉起上游子进程，并启动读取线程把上游 stdout 行推入 `tx`。
    pub fn spawn(
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
        tx: Sender<String>,
    ) -> io::Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("启动上游 MCP server `{command}` 失败：{e}"),
            )
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("无法获取上游 stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("无法获取上游 stdout"))?;

        let reader = thread::spawn(move || {
            let mut r = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match r.read_line(&mut line) {
                    Ok(0) | Err(_) => break, // 上游关闭或出错
                    Ok(_) => {
                        let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if tx.send(trimmed).is_err() {
                            break; // 消费端已走
                        }
                    }
                }
            }
            // 线程结束时 tx 被丢弃，channel 随之关闭
        });

        Ok(StdioTransport {
            child,
            stdin: Some(stdin),
            reader: Some(reader),
        })
    }
}

impl Transport for StdioTransport {
    fn send(&mut self, line: &str) -> io::Result<()> {
        match &mut self.stdin {
            Some(w) => {
                writeln!(w, "{line}")?;
                w.flush()
            }
            None => Err(io::Error::other("上游 stdin 已关闭")),
        }
    }

    fn close(&mut self) {
        // 关闭 stdin → 上游收到 EOF 自然退出 → 读取线程读到 EOF 结束 → channel 关闭
        self.stdin.take();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        let _ = self.child.wait();
    }
}
