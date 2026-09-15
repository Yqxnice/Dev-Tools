use thiserror::Error;

/// 应用统一错误类型（仅包含实际会构造的变体）
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("命令执行失败: {0}")]
    CommandExecution(String),

    #[error("服务名无效: {0}")]
    InvalidServiceName(String),

    #[error("验证失败: {0}")]
    Validation(String),

    #[error("网络错误: {0}")]
    Network(String),
}

impl AppError {
    /// 转换为用户友好的错误消息
    pub fn to_user_message(&self) -> String {
        match self {
            AppError::Io(e) => format!("文件操作失败: {}", e),
            AppError::CommandExecution(cmd) => format!("命令执行失败: {}", cmd),
            AppError::InvalidServiceName(name) => format!("服务名无效: {}", name),
            AppError::Validation(msg) => format!("验证失败: {}", msg),
            AppError::Network(msg) => format!("网络请求失败: {}", msg),
        }
    }
}

/// 为 Tauri 命令实现 From<AppError> for String
impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_user_message()
    }
}

/// 便捷结果类型
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let err = AppError::InvalidServiceName("test| malicious".to_string());
        let msg = err.to_user_message();
        assert!(msg.contains("服务名无效"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_user_message().contains("文件操作失败"));
    }
}
