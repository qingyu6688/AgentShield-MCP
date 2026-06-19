//! 把一次 MCP 工具调用解析为 [`ToolCall`]：推断事件类型、提取目标资源。

use agentshield_core::{ids, EventType, ToolCall};
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;

static SQL_KEYWORD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(select|insert|update|delete|drop|truncate|alter|create)\b").unwrap()
});

/// 从 tools/call 的参数解析出一次调用。
///
/// - `tool_name`：MCP 工具名
/// - `arguments`：工具参数对象
pub fn classify(
    session_id: &str,
    client_name: &str,
    server_name: &str,
    tool_name: &str,
    arguments: serde_json::Value,
) -> ToolCall {
    let event_type = infer_event_type(tool_name, &arguments);
    let target = extract_target(event_type, &arguments);

    ToolCall {
        id: ids::new_event_id(),
        session_id: session_id.to_string(),
        client_name: client_name.to_string(),
        server_name: server_name.to_string(),
        tool_name: tool_name.to_string(),
        event_type,
        target,
        arguments,
        created_at: Utc::now(),
    }
}

/// 按参数强信号 + 工具名模式推断事件类型。
///
/// 优先看参数里的强信号（command 参数、含 SQL 关键字的 query 参数），
/// 因为工具名启发式容易误判——例如名为 `execute` 的数据库工具会被 `exec` 模式
/// 误当成 shell。
pub fn infer_event_type(tool_name: &str, args: &serde_json::Value) -> EventType {
    let name = tool_name.to_ascii_lowercase();

    // 1. 参数强信号
    if has_key(args, &["command", "cmd"]) {
        return EventType::ShellExec;
    }
    if let Some(s) = get_str(args, &["sql", "query", "statement"]) {
        if SQL_KEYWORD.is_match(s) {
            return EventType::DbQuery;
        }
    }

    // 2. 工具名模式（配合参数特征确定文件操作）
    let has_path = has_key(args, &["path", "file", "filename"]);
    if has_path && (name.contains("delete") || name.contains("remove") || name.contains("rm")) {
        return EventType::FileDelete;
    }
    if name.contains("rename") || name.contains("move") {
        return EventType::FileRename;
    }
    if has_path && (name.contains("write") || name.contains("edit") || name.contains("create")) {
        return EventType::FileWrite;
    }
    if has_path && (name.contains("read") || name.contains("get") || name.contains("cat")) {
        return EventType::FileRead;
    }
    if name.contains("exec")
        || name.contains("shell")
        || name.contains("command")
        || name.contains("run")
    {
        return EventType::ShellExec;
    }
    if name.contains("query") || name.contains("sql") {
        return EventType::DbQuery;
    }

    // 3. 参数弱信号
    if has_key(args, &["url", "uri"]) {
        return EventType::NetworkRequest;
    }
    if has_key(args, &["path", "file", "filename"]) {
        return EventType::FileRead;
    }

    EventType::McpToolCall
}

/// 提取目标资源：文件取路径，shell 取命令，db 取 SQL。
pub fn extract_target(et: EventType, args: &serde_json::Value) -> Option<String> {
    match et {
        EventType::FileRead
        | EventType::FileWrite
        | EventType::FileDelete
        | EventType::FileRename => get_str(args, &["path", "file", "filename"]).map(str::to_string),
        EventType::ShellExec => get_str(args, &["command", "cmd"]).map(str::to_string),
        EventType::DbQuery => get_str(args, &["sql", "query", "statement"]).map(str::to_string),
        EventType::NetworkRequest => get_str(args, &["url", "uri"]).map(str::to_string),
        _ => None,
    }
}

fn has_key(args: &serde_json::Value, keys: &[&str]) -> bool {
    keys.iter().any(|k| args.get(k).is_some())
}

fn get_str<'a>(args: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| args.get(k).and_then(|v| v.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infers_file_read() {
        let et = infer_event_type("read_file", &json!({ "path": "./a.txt" }));
        assert_eq!(et, EventType::FileRead);
    }

    #[test]
    fn infers_shell_from_command_arg() {
        let et = infer_event_type("anything", &json!({ "command": "ls -la" }));
        assert_eq!(et, EventType::ShellExec);
    }

    #[test]
    fn infers_db_from_sql() {
        let et = infer_event_type("execute", &json!({ "query": "DROP TABLE users" }));
        assert_eq!(et, EventType::DbQuery);
    }

    #[test]
    fn extracts_command_target() {
        let call = classify(
            "s",
            "c",
            "srv",
            "shell.exec",
            json!({ "command": "rm -rf /" }),
        );
        assert_eq!(call.event_type, EventType::ShellExec);
        assert_eq!(call.target.as_deref(), Some("rm -rf /"));
    }

    #[test]
    fn unknown_falls_back_to_mcp_call() {
        let et = infer_event_type("list_repos", &json!({ "owner": "x" }));
        assert_eq!(et, EventType::McpToolCall);
    }
}
