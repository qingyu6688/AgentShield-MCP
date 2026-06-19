//! 核心错误类型。

/// core 层错误。库内统一返回 `Result`，不 panic。
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("配置文件读取失败: {0}")]
    ConfigIo(#[from] std::io::Error),

    #[error("配置解析失败: {0}")]
    ConfigParse(String),

    #[error("配置校验失败: {0}")]
    ConfigInvalid(String),
}

/// 便捷 Result 别名。
pub type Result<T> = std::result::Result<T, CoreError>;
