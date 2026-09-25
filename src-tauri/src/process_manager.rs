use super::error::{AppError, AppResult};
use super::types::ProcessOutput;
use std::collections::HashMap;
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

/// 执行 PowerShell 脚本，动态参数通过环境变量传入（避免字符串拼接导致的命令注入）
pub async fn execute_powershell_env(
    script: &str,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
) -> AppResult<ProcessOutput> {
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    for (key, value) in env_vars {
        command.env(key, value);
    }

    #[cfg(target_os = "windows")]
    hide_console_window(&mut command);

    let output = timeout(Duration::from_secs(timeout_secs), command.output())
        .await
        .map_err(|_| AppError::CommandExecution(format!("PowerShell 执行超时 ({}秒)", timeout_secs)))?
        .map_err(|e| AppError::CommandExecution(format!("启动 PowerShell 失败: {}", e)))?;

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// 终止可执行文件位于 bin_dir 目录下的 mysqld.exe / mysql.exe 进程。
/// 按可执行文件路径精准匹配实例，避免误杀同机其他 MySQL/MariaDB 实例。返回终止的进程数。
pub async fn kill_mysql_processes_in_dir(bin_dir: &str) -> AppResult<usize> {
    let mut env_vars = HashMap::new();
    env_vars.insert(
        "MYSQL_BIN_DIR".to_string(),
        bin_dir.trim_end_matches('\\').to_string(),
    );

    let script = r#"
        $bin = $env:MYSQL_BIN_DIR
        if (-not $bin) { Write-Output 0; exit 0 }
        $count = 0
        Get-CimInstance Win32_Process -Filter "Name='mysqld.exe' OR Name='mysql.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like "$bin\*" } |
            ForEach-Object {
                try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop; $count++ } catch {}
            }
        Write-Output $count
    "#;

    let output = execute_powershell_env(script, &env_vars, 60).await?;
    // 区分"0 个进程"与"输出异常"：parse 失败时返回 Err 而非静默 0，
    // 避免 count 调用方误以为进程已退出而提前结束等待
    let last_line = output.stdout.trim().lines().last().unwrap_or("").trim();
    let count: usize = last_line.parse().map_err(|_| {
        AppError::CommandExecution(format!("无法解析进程数输出: {}", last_line))
    })?;
    Ok(count)
}

/// 统计 bin_dir 目录下仍在运行的 mysqld/mysql 进程数（用于终止后轮询等待进程退出）
pub async fn count_mysql_processes_in_dir(bin_dir: &str) -> AppResult<usize> {
    let mut env_vars = HashMap::new();
    env_vars.insert(
        "MYSQL_BIN_DIR".to_string(),
        bin_dir.trim_end_matches('\\').to_string(),
    );

    let script = r#"
        $bin = $env:MYSQL_BIN_DIR
        if (-not $bin) { Write-Output 0; exit 0 }
        $count = 0
        Get-CimInstance Win32_Process -Filter "Name='mysqld.exe' OR Name='mysql.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like "$bin\*" } |
            ForEach-Object { $count++ }
        Write-Output $count
    "#;

    let output = execute_powershell_env(script, &env_vars, 60).await?;
    // 区分"0 个进程"与"输出异常"：parse 失败时返回 Err 而非静默 0，
    // 避免 count 调用方误以为进程已退出而提前结束等待
    let last_line = output.stdout.trim().lines().last().unwrap_or("").trim();
    let count: usize = last_line.parse().map_err(|_| {
        AppError::CommandExecution(format!("无法解析进程数输出: {}", last_line))
    })?;
    Ok(count)
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

/// 验证密码强度
pub fn validate_password_strength(password: &str) -> AppResult<()> {
    if password.len() < 6 {
        return Err(AppError::Validation("密码长度至少需要6个字符".to_string()));
    }
    if password.len() > 128 {
        return Err(AppError::Validation("密码长度不能超过128个字符".to_string()));
    }

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
    if url.contains([';', '|', '&', '`', '$']) {
        return Err(AppError::Validation("URL包含非法字符".to_string()));
    }
    // 阻止命令注入、换行符注入和 null 字节
    if url.contains('\n') || url.contains('\r') || url.contains('\0') {
        return Err(AppError::Validation("URL包含非法控制字符".to_string()));
    }
    if url.contains("$(") || url.contains("${") {
        return Err(AppError::Validation("URL包含命令替换语法".to_string()));
    }
    // 阻止路径穿越
    if url.contains("..") {
        return Err(AppError::Validation("URL包含路径穿越".to_string()));
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
        assert!(validate_password_strength("abc12").is_err()); // 太短（<6）
        assert!(validate_password_strength("abc123").is_ok()); // 刚好6位
        assert!(validate_password_strength("abcdefgh").is_ok()); // 只有字母但长度足够
        assert!(validate_password_strength("Abcdefg1").is_ok());
        assert!(validate_password_strength("Abc@1234").is_ok());
    }
}
