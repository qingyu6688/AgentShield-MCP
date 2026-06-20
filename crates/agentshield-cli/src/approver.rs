//! 终端确认。
//!
//! 注意：stdio 代理模式下，进程的 stdin/stdout 已被 MCP 协议占用，
//! 确认 UI 不能用它们。这里直接打开控制终端设备（Unix 的 `/dev/tty`、
//! Windows 的 `CONIN$` / `CONOUT$`）来交互。拿不到终端时（如 CI 无 tty），
//! 按配置的超时兜底动作处理——默认拒绝，更安全。
//!
//! 用户选「始终允许 / 永久拉黑」时，结果写入持久化决策记忆，
//! 下次同一操作由决策器直接放行 / 拦截，不再打扰。

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;

use agentshield_core::{Decision, DecisionMemory, ToolCall};
use agentshield_proxy::{ApprovalResult, Approver};

/// 无 tty 时的兜底动作。
#[derive(Debug, Clone, Copy)]
pub enum FallbackAction {
    Deny,
    Allow,
}

pub struct CliApprover {
    fallback: FallbackAction,
    memory: Option<Arc<DecisionMemory>>,
}

impl CliApprover {
    /// 带持久化记忆：用户选「始终允许 / 永久拉黑」时落盘。
    pub fn with_memory(fallback: FallbackAction, memory: Arc<DecisionMemory>) -> Self {
        CliApprover {
            fallback,
            memory: Some(memory),
        }
    }

    fn fallback_result(&self) -> ApprovalResult {
        match self.fallback {
            FallbackAction::Deny => ApprovalResult::TimedOut,
            FallbackAction::Allow => ApprovalResult::AllowOnce,
        }
    }
}

impl Approver for CliApprover {
    fn approve(&self, call: &ToolCall, decision: &Decision) -> ApprovalResult {
        let Some((mut out, reader)) = open_tty() else {
            return self.fallback_result();
        };

        let prompt = format!(
            "\n  AgentShield · 需要确认\n\n  来源     {}\n  操作     {:?}\n  目标     {}\n  风险     {:?}  {}/100\n  原因     {}\n\n  [y] 允许一次   [a] 始终允许   [n] 拒绝   [b] 永久拉黑\n  > ",
            call.client_name,
            call.event_type,
            call.target.as_deref().unwrap_or("-"),
            decision.risk.level,
            decision.risk.score,
            decision.reason,
        );
        let _ = out.write_all(prompt.as_bytes());
        let _ = out.flush();

        let mut line = String::new();
        let mut reader = reader;
        if reader.read_line(&mut line).is_err() {
            return self.fallback_result();
        }

        match line.trim().to_ascii_lowercase().as_str() {
            "y" => ApprovalResult::AllowOnce,
            "a" => {
                if let Some(mem) = &self.memory {
                    mem.remember_allow(call);
                }
                ApprovalResult::AllowAlways
            }
            "b" => {
                if let Some(mem) = &self.memory {
                    mem.remember_block(call);
                }
                ApprovalResult::BlockForever
            }
            _ => ApprovalResult::Deny,
        }
    }
}

/// 打开控制终端，返回（写端, 读端）。失败返回 None。
#[cfg(unix)]
fn open_tty() -> Option<(Box<dyn Write>, Box<dyn BufRead>)> {
    let write = OpenOptions::new().write(true).open("/dev/tty").ok()?;
    let read = OpenOptions::new().read(true).open("/dev/tty").ok()?;
    Some((Box::new(write), Box::new(BufReader::new(read))))
}

#[cfg(windows)]
fn open_tty() -> Option<(Box<dyn Write>, Box<dyn BufRead>)> {
    let write = OpenOptions::new().write(true).open("CONOUT$").ok()?;
    let read = OpenOptions::new().read(true).open("CONIN$").ok()?;
    Some((Box::new(write), Box::new(BufReader::new(read))))
}
