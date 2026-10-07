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

/// 执行命令，并把 `stdin_data` 经管道写入子进程标准输入。
///
/// **为什么需要它**：Windows 上同用户会话内的任意进程都能通过
/// `Win32_Process.CommandLine` 读到别的进程的完整命令行。SQL 中若含密码
/// （如 `ALTER USER ... IDENTIFIED BY 'xxx'`）走 `-e` 就等于把密码广播给
/// 本机所有进程；`psql`/`mysql` 的 `-p<密码>` 同理。改走 stdin 后参数里
/// 只剩不含秘密的连接信息。
///
/// stdout/stderr 照常被捕获，语义与 [`execute_command_with_timeout`] 一致。
pub async fn execute_command_with_stdin(
    cmd: &str,
    args: &[&str],
    stdin_data: &str,
    timeout_secs: u64,
) -> AppResult<ProcessOutput> {
    use tokio::io::AsyncWriteExt;

    let mut command = Command::new(cmd);
    command.args(args);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    hide_console_window(&mut command);

    let mut child = command
        .spawn()
        .map_err(|e| AppError::CommandExecution(format!("启动命令失败: {}", e)))?;

    // 写 stdin 失败（子进程提前退出）不视为致命错误：
    // 真正的失败信息由下面的 wait_with_output 收集到 stderr
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_data.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    let output = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
        .await
        .map_err(|_| AppError::CommandExecution(format!("命令执行超时 ({}秒)", timeout_secs)))?
        .map_err(|e| AppError::CommandExecution(format!("等待命令输出失败: {}", e)))?;

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
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
    // 控制字符无法安全地表达在 MySQL 选项文件 / 连接串 / 脚本里，
    // 一旦被写坏会变成"以为设置了密码，实际没设置"的静默故障 → 一律拒绝
    if password.chars().any(|c| c.is_control()) {
        return Err(AppError::Validation(
            "密码不能包含控制字符（换行、制表符、NUL 等）".to_string(),
        ));
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

    #[test]
    fn validate_password_strength_rejects_control_chars() {
        // 选项文件 / 连接串无法安全表达控制字符，必须整体拒绝（fail closed）
        assert!(validate_password_strength("abcdef\n").is_err());
        assert!(validate_password_strength("abcdef\r").is_err());
        assert!(validate_password_strength("abc\ndef123").is_err());
        assert!(validate_password_strength("abcdef\t").is_err());
        assert!(validate_password_strength("abc\0def").is_err());
    }

    #[test]
    fn validate_password_strength_rejects_overlong() {
        assert!(validate_password_strength("a".repeat(129).as_str()).is_err());
        assert!(validate_password_strength("a".repeat(128).as_str()).is_ok());
    }

    /// stdin 走管道，不占命令行参数 —— 这是隐藏 SQL/密码的关键路径
    #[tokio::test]
    async fn execute_command_with_stdin_feeds_child_process() {
        let out = execute_command_with_stdin("findstr", &["^"], "hello\r\nworld\r\n", 10)
            .await
            .expect("命令应能执行");
        assert_eq!(out.exit_code, 0, "findstr 匹配到行时退出码应为 0");
        assert!(out.stdout.contains("hello"), "stdout: {}", out.stdout);
        assert!(out.stdout.contains("world"), "stdout: {}", out.stdout);
    }

    #[tokio::test]
    async fn execute_command_with_stdin_reports_nonexistent_binary() {
        let err = execute_command_with_stdin(
            "definitely-not-existing-xyz-98765",
            &[],
            "",
            5,
        )
        .await
        .expect_err("不存在的命令应返回 Err");
        assert!(err.to_string().contains("启动命令失败"));
    }
}
