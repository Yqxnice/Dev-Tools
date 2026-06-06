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

pub async fn execute_command_with_timeout(
    cmd: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<ProcessOutput, String> {
    let mut command = Command::new(cmd);
    command.args(args);

    #[cfg(target_os = "windows")]
    hide_console_window(&mut command);

    let output = timeout(Duration::from_secs(timeout_secs), command.output())
        .await
        .map_err(|e| format!("命令执行超时: {}", e))?
        .map_err(|e| format!("启动命令失败: {}", e))?;

    Ok(ProcessOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

pub async fn execute_command(cmd: &str, args: &[&str]) -> Result<ProcessOutput, String> {
    execute_command_with_timeout(cmd, args, 30).await
}
