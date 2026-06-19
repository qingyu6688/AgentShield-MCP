//! 审计层错误类型。

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("审计 IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite 操作失败: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON 序列化失败: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AuditError>;
