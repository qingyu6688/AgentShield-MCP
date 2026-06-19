//! id 生成。事件 id 与会话 id。

use uuid::Uuid;

/// 生成一个新的事件 id。
pub fn new_event_id() -> String {
    Uuid::new_v4().to_string()
}

/// 生成一个新的会话 id。
pub fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}
