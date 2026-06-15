use thiserror::Error;

/// 应用统一错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP 请求错误: {0}")]
    Http(#[from] reqwest::Error),

    #[error("正则表达式错误: {0}")]
    Regex(#[from] regex::Error),

    #[error("Tauri 错误: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("命令执行失败: {0}")]
    CommandExecution(String),

    #[error("服务名无效: {0}")]
    InvalidServiceName(String),

    #[error("路径无效: {0}")]
    InvalidPath(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("检测失败: {0}")]
    DetectionFailed(String),

    #[error("操作失败: {0}")]
    OperationFailed(String),

    #[error("缓存错误: {0}")]
    Cache(String),

    #[error("验证失败: {0}")]
    Validation(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("下载错误: {0}")]
    Download(String),

    #[error("安装错误: {0}")]
    Installation(String),

    #[error("卸载错误: {0}")]
    Uninstallation(String),

    #[error("清理错误: {0}")]
    Cleanup(String),

    #[error("密码操作错误: {0}")]
    PasswordOperation(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

impl AppError {
    /// 转换为用户友好的错误消息
    pub fn to_user_message(&self) -> String {
        match self {
            AppError::Io(e) => format!("文件操作失败: {}", e),
            AppError::Json(e) => format!("数据解析失败: {}", e),
            AppError::Http(e) => format!("网络请求失败: {}", e),
            AppError::Regex(e) => format!("正则匹配失败: {}", e),
            AppError::Tauri(e) => format!("应用错误: {}", e),
            AppError::CommandExecution(cmd) => format!("命令执行失败: {}", cmd),
            AppError::InvalidServiceName(name) => format!("服务名无效: {}", name),
            AppError::InvalidPath(path) => format!("路径无效: {}", path),
            AppError::PermissionDenied(msg) => format!("权限不足: {}", msg),
            AppError::Config(msg) => format!("配置错误: {}", msg),
            AppError::DetectionFailed(msg) => format!("检测失败: {}", msg),
            AppError::OperationFailed(msg) => format!("操作失败: {}", msg),
            AppError::Cache(msg) => format!("缓存错误: {}", msg),
            AppError::Validation(msg) => format!("验证失败: {}", msg),
            AppError::Network(msg) => format!("网络错误: {}", msg),
            AppError::Download(msg) => format!("下载失败: {}", msg),
            AppError::Installation(msg) => format!("安装失败: {}", msg),
            AppError::Uninstallation(msg) => format!("卸载失败: {}", msg),
            AppError::Cleanup(msg) => format!("清理失败: {}", msg),
            AppError::PasswordOperation(msg) => format!("密码操作失败: {}", msg),
            AppError::Unknown(msg) => format!("未知错误: {}", msg),
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
