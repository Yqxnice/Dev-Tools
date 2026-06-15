use super::error::{AppError, AppResult};
use super::types::ProcessOutput;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

// 在 Windows 上隐藏控制台窗口，避免闪烁
#[cfg(target_os = "windows")]
fn hide_console_window(command: &mut Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
}

/// 执行命令并设置超时
pub async fn execute_command_with_timeout(
    cmd: &str,
    args: &[&str],
    timeout_secs: u64,
) -> AppResult<ProcessOutput> {
    let mut command = Command::new(cmd);
    command.args(args);

    #[cfg(target_os = "windows")]
    hide_console_window(&mut command);

    let output = timeout(Duration::from_secs(timeout_secs), command.output())
        .await
        .map_err(|_| AppError::CommandExecution(format!("命令执行超时 ({}秒)", timeout_secs)))?
        .map_err(|e| AppError::CommandExecution(format!("启动命令失败: {}", e)))?;

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// 执行命令（默认30秒超时）
pub async fn execute_command(cmd: &str, args: &[&str]) -> AppResult<ProcessOutput> {
    execute_command_with_timeout(cmd, args, 30).await
}

/// 执行命令并返回成功/失败状态
pub async fn execute_command_success(cmd: &str, args: &[&str]) -> AppResult<bool> {
    let output = execute_command(cmd, args).await?;
    Ok(output.exit_code == 0)
}

/// 验证服务名是否安全
/// 
/// 规则：
/// - 不能为空
/// - 长度不超过256
/// - 只允许字母、数字、连字符、下划线、点号
/// - 不能包含空格或特殊字符
pub fn validate_service_name(name: &str) -> AppResult<()> {
    if name.is_empty() {
        return Err(AppError::InvalidServiceName("服务名不能为空".to_string()));
    }
    if name.len() > 256 {
        return Err(AppError::InvalidServiceName("服务名长度不能超过256".to_string()));
    }
    if name.contains(|c: char| c.is_whitespace()) {
        return Err(AppError::InvalidServiceName("服务名不能包含空格".to_string()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(AppError::InvalidServiceName(format!(
            "服务名只能包含字母、数字、连字符、下划线、点号: {}",
            name
        )));
    }
    Ok(())
}

/// 验证文件路径
pub fn validate_path(path: &str, allow_relative: bool) -> AppResult<()> {
    if path.is_empty() {
        return Err(AppError::InvalidPath("路径不能为空".to_string()));
    }
    if path.len() > 260 {
        return Err(AppError::InvalidPath("路径长度超过Windows限制(260)".to_string()));
    }
    if path.contains("..") {
        return Err(AppError::InvalidPath("路径不能包含..".to_string()));
    }
    if !allow_relative && !std::path::Path::new(path).is_absolute() {
        return Err(AppError::InvalidPath("路径必须是绝对路径".to_string()));
    }
    Ok(())
}

/// 验证端口号
pub fn validate_port(port: u16) -> AppResult<()> {
    if port == 0 {
        return Err(AppError::Validation("端口号不能为0".to_string()));
    }
    Ok(())
}

/// 验证MySQL主机名
pub fn validate_mysql_host(host: &str) -> AppResult<()> {
    if host.is_empty() {
        return Err(AppError::Validation("主机名不能为空".to_string()));
    }
    if host.len() > 253 {
        return Err(AppError::Validation("主机名过长".to_string()));
    }
    if !host.chars().all(|c| 
        c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == '%'
    ) {
        return Err(AppError::Validation(format!(
            "主机名包含非法字符: {}",
            host
        )));
    }
    Ok(())
}

/// 验证密码强度
pub fn validate_password_strength(password: &str) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::Validation("密码长度至少需要8个字符".to_string()));
    }
    if password.len() > 128 {
        return Err(AppError::Validation("密码长度不能超过128个字符".to_string()));
    }
    
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());
    
    let strength = [has_upper, has_lower, has_digit, has_special]
        .iter()
        .filter(|&&x| x)
        .count();
    
    if strength < 2 {
        return Err(AppError::Validation(
            "密码需包含大写字母、小写字母、数字、特殊字符中的至少2种".to_string()
        ));
    }
    
    Ok(())
}

/// 验证Python路径
pub fn validate_python_path(path: &str) -> AppResult<()> {
    validate_path(path, false)?;
    
    let lower = path.to_lowercase();
    if !lower.ends_with("python.exe") && !lower.ends_with("python3.exe") {
        return Err(AppError::InvalidPath("路径必须指向python.exe或python3.exe".to_string()));
    }
    
    Ok(())
}

/// 验证安装路径
pub fn validate_install_path(path: &str) -> AppResult<()> {
    validate_path(path, false)?;
    Ok(())
}

/// 验证镜像源URL
pub fn validate_mirror_url(url: &str) -> AppResult<()> {
    if url.is_empty() {
        return Err(AppError::Validation("镜像源URL不能为空".to_string()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::Validation("镜像源URL必须以http://或https://开头".to_string()));
    }
    if url.contains(|c: char| c == ';' || c == '|' || c == '&' || c == '`') {
        return Err(AppError::Validation("URL包含非法字符".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_service_name_valid() {
        assert!(validate_service_name("MySQL80").is_ok());
        assert!(validate_service_name("my-service_v1.0").is_ok());
    }

    #[test]
    fn test_validate_service_name_invalid() {
        assert!(validate_service_name("").is_err());
        assert!(validate_service_name("service name").is_err());
        assert!(validate_service_name("service;rm -rf").is_err());
        assert!(validate_service_name("a".repeat(257).as_str()).is_err());
    }

    #[test]
    fn test_validate_password_strength() {
        assert!(validate_password_strength("abc123").is_err()); // 太短
        assert!(validate_password_strength("abcdefgh").is_err()); // 只有字母
        assert!(validate_password_strength("Abcdefg1").is_ok()); // 包含大小写和数字
        assert!(validate_password_strength("Abc@1234").is_ok()); // 包含所有类型
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("", false).is_err());
        assert!(validate_path("relative/path", false).is_err());
        assert!(validate_path(r"C:\valid\path", false).is_ok());
        assert!(validate_path(r"C:\path\..\other", false).is_err());
    }

    #[test]
    fn test_validate_mysql_host() {
        assert!(validate_mysql_host("localhost").is_ok());
        assert!(validate_mysql_host("127.0.0.1").is_ok());
        assert!(validate_mysql_host("%").is_ok());
        assert!(validate_mysql_host("host;rm -rf").is_err());
    }
}
