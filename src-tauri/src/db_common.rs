//! MySQL / PostgreSQL 共享的底层执行与等待原语。
//!
//! 只收敛两侧逐字节等价的 A 类 helper（PowerShell 执行、服务/端口轮询），
//! 数据库特有流程（注册表扫描脚本、密码重置主流程）仍保留在各自模块中——
//! 强行泛化 B 类逻辑（匹配规则、脚本内容不同）会增加维护风险。

use crate::{detector_base, logger};
use std::collections::HashMap;
use tauri::AppHandle;

/// 使用环境变量传递参数执行 PowerShell 脚本，避免字符串注入。
///
/// 动态参数一律通过 `env_vars` 注入（脚本内以 `$env:KEY` 读取），
/// 绝不拼接进脚本字符串。返回 stdout；执行失败或超时（30 秒）返回空串。
pub async fn run_powershell_with_env(script: &str, env_vars: &HashMap<String, String>) -> String {
    let mut command = tokio::process::Command::new("powershell");
    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    for (key, value) in env_vars {
        command.env(key, value);
    }
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW，避免控制台窗口闪烁
    match tokio::time::timeout(std::time::Duration::from_secs(30), command.output()).await {
        Ok(Ok(output)) => String::from_utf8_lossy(&output.stdout).to_string(),
        Ok(Err(_)) => String::new(),
        Err(_) => String::new(), // timeout
    }
}

/// 使用环境变量执行 PowerShell 脚本并返回非空行
pub async fn run_powershell_lines_with_env(
    script: &str,
    env_vars: &HashMap<String, String>,
) -> Vec<String> {
    let output = run_powershell_with_env(script, env_vars).await;
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// 轮询等待服务达到目标状态（running=true 等待"启动"，running=false 等待"停止"/"未安装"）
pub async fn wait_for_service_state(
    app_handle: &AppHandle,
    service_name: &str,
    running: bool,
    timeout_secs: u64,
) -> bool {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let status = crate::service_manager::check_service_status(service_name).await;
        let reached = if running {
            status == detector_base::STATUS_RUNNING
        } else {
            status != detector_base::STATUS_RUNNING
        };
        if reached {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            logger::warn(
                app_handle,
                &format!("等待服务 {} 状态超时（当前状态: {}）", service_name, status),
            );
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// 轮询等待端口可连接（用于确认服务已就绪接受连接）
pub async fn wait_for_port_ready(app_handle: &AppHandle, port: u16, timeout_secs: u64) -> bool {
    use tokio::net::TcpStream;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let connect = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            TcpStream::connect(("127.0.0.1", port)),
        )
        .await;
        if matches!(connect, Ok(Ok(_))) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            logger::warn(app_handle, &format!("等待端口 {} 就绪超时", port));
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
