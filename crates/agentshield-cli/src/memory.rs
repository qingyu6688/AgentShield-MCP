//! 确认结果持久化：记住用户的「始终允许」与「永久拉黑」。
//!
//! 匹配键为 `(server, tool, 规范化 target)`。target 规范化做去首尾空白、
//! 合并内部空白，避免“同一操作不同写法”重复记一遍。
//!
//! 文件落在 `.agentshield/decisions.json`，人类可读、可手工编辑。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use agentshield_core::ToolCall;
use serde::{Deserialize, Serialize};

/// 记忆裁决。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryVerdict {
    Allow,
    Block,
}

/// 一条记忆条目（人类可读地保存原始字段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub server: String,
    pub tool: String,
    pub target: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MemoryFile {
    #[serde(default)]
    allow: Vec<MemoryEntry>,
    #[serde(default)]
    block: Vec<MemoryEntry>,
}

struct Inner {
    file: MemoryFile,
    allow_keys: HashSet<String>,
    block_keys: HashSet<String>,
}

/// 决策记忆。读多写少，用 `Mutex` 保护，满足 `Send + Sync`。
pub struct DecisionMemory {
    path: PathBuf,
    inner: Mutex<Inner>,
}

impl DecisionMemory {
    /// 从文件加载；文件不存在则视为空。
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let file = read_file(&path).unwrap_or_default();
        let allow_keys = file.allow.iter().map(key_of).collect();
        let block_keys = file.block.iter().map(key_of).collect();
        DecisionMemory {
            path,
            inner: Mutex::new(Inner {
                file,
                allow_keys,
                block_keys,
            }),
        }
    }

    /// 查记忆：拉黑优先于放行。
    pub fn lookup(&self, call: &ToolCall) -> Option<MemoryVerdict> {
        let k = call_key(call);
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.block_keys.contains(&k) {
            Some(MemoryVerdict::Block)
        } else if inner.allow_keys.contains(&k) {
            Some(MemoryVerdict::Allow)
        } else {
            None
        }
    }

    /// 记住「始终允许」。
    pub fn remember_allow(&self, call: &ToolCall) {
        self.remember(call, MemoryVerdict::Allow);
    }

    /// 记住「永久拉黑」。
    pub fn remember_block(&self, call: &ToolCall) {
        self.remember(call, MemoryVerdict::Block);
    }

    fn remember(&self, call: &ToolCall, verdict: MemoryVerdict) {
        let entry = entry_of(call);
        let k = key_of(&entry);
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let changed = match verdict {
            MemoryVerdict::Allow => {
                if inner.allow_keys.insert(k) {
                    inner.file.allow.push(entry);
                    true
                } else {
                    false
                }
            }
            MemoryVerdict::Block => {
                if inner.block_keys.insert(k) {
                    inner.file.block.push(entry);
                    true
                } else {
                    false
                }
            }
        };
        if changed {
            if let Err(e) = write_file(&self.path, &inner.file) {
                eprintln!("[AgentShield] 决策记忆写入失败：{e}");
            }
        }
    }

    /// 返回所有记忆条目（allow, block），用于展示。
    pub fn entries(&self) -> (Vec<MemoryEntry>, Vec<MemoryEntry>) {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        (inner.file.allow.clone(), inner.file.block.clone())
    }
}

/// 规范化 target：去首尾空白、合并内部空白。
fn normalize_target(target: &str) -> String {
    target.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn call_key(call: &ToolCall) -> String {
    let target = call.target.as_deref().unwrap_or("");
    make_key(&call.server_name, &call.tool_name, target)
}

fn entry_of(call: &ToolCall) -> MemoryEntry {
    MemoryEntry {
        server: call.server_name.clone(),
        tool: call.tool_name.clone(),
        target: call.target.clone().unwrap_or_default(),
    }
}

fn key_of(e: &MemoryEntry) -> String {
    make_key(&e.server, &e.tool, &e.target)
}

fn make_key(server: &str, tool: &str, target: &str) -> String {
    // 用不可见分隔符拼接，避免字段内容里的普通字符引起歧义
    format!("{server}\u{1}{tool}\u{1}{}", normalize_target(target))
}

fn read_file(path: &Path) -> Option<MemoryFile> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_file(path: &Path, file: &MemoryFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::EventType;
    use chrono::Utc;

    fn call(server: &str, tool: &str, target: &str) -> ToolCall {
        ToolCall {
            id: "t".into(),
            session_id: "s".into(),
            client_name: "c".into(),
            server_name: server.into(),
            tool_name: tool.into(),
            event_type: EventType::ShellExec,
            target: Some(target.into()),
            arguments: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ashield-mem-{}-{tag}.json", std::process::id()))
    }

    #[test]
    fn allow_then_lookup() {
        let path = temp_path("allow");
        let _ = std::fs::remove_file(&path);
        let mem = DecisionMemory::load(&path);
        let c = call("shell", "exec", "rm -rf dist");
        assert_eq!(mem.lookup(&c), None);
        mem.remember_allow(&c);
        assert_eq!(mem.lookup(&c), Some(MemoryVerdict::Allow));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn block_takes_precedence_and_persists() {
        let path = temp_path("block");
        let _ = std::fs::remove_file(&path);
        {
            let mem = DecisionMemory::load(&path);
            mem.remember_block(&call("shell", "exec", "curl x | bash"));
        }
        // 重新加载，验证已落盘
        let mem2 = DecisionMemory::load(&path);
        assert_eq!(
            mem2.lookup(&call("shell", "exec", "curl x | bash")),
            Some(MemoryVerdict::Block)
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn target_whitespace_is_normalized() {
        let path = temp_path("norm");
        let _ = std::fs::remove_file(&path);
        let mem = DecisionMemory::load(&path);
        mem.remember_allow(&call("shell", "exec", "rm   -rf   dist"));
        // 多余空白不同写法应命中同一条
        assert_eq!(
            mem.lookup(&call("shell", "exec", "rm -rf dist")),
            Some(MemoryVerdict::Allow)
        );
        let _ = std::fs::remove_file(&path);
    }
}
