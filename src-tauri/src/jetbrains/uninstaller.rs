use super::super::{logger, process_manager, types::JetBrainsInstallation};
use std::collections::HashMap;
use tauri::AppHandle;

/// 终止该 IDE 的运行进程（按可执行文件路径精准匹配，避免误杀其他 IDE）
async fn kill_ide_processes(app_handle: &AppHandle, install_location: &str) {
    if install_location.is_empty() {
        return;
    }
    let mut env_vars = HashMap::new();
    env_vars.insert("IDE_DIR".to_string(), install_location.trim_end_matches('\\').to_string());

    let script = r#"
        $dir = $env:IDE_DIR
        if (-not $dir) { Write-Output 0; exit 0 }
        $count = 0
        Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
            Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like "$dir\*" } |
            ForEach-Object {
                try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop; $count++ } catch {}
            }
        Write-Output $count
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 60).await {
        Ok(out) => {
            let n: usize = out.stdout.trim().lines().last().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
            if n > 0 {
                logger::info(&app_handle, &format!("已终止 {} 个 IDE 进程", n));
            }
        }
        Err(e) => logger::warn(&app_handle, &format!("终止 IDE 进程异常: {}", e)),
    }
}

/// 卸载选中的 JetBrains 产品。
///
/// 优先使用 QuietUninstallString（静默）；退而求其次用 UninstallString 并尝试加静默参数。
/// MSI 安装器使用 msiexec /x {ProductCode} /quiet。
pub async fn uninstall_jetbrains(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> Result<(), String> {
    logger::info(
        &app_handle,
        &format!("开始卸载 {}", installation.product_name),
    );

    // 1. 先终止该 IDE 进程，释放文件占用
    kill_ide_processes(&app_handle, &installation.install_location).await;

    // 2. 执行卸载命令
    let uninstall_cmd = if !installation.quiet_uninstall_string.is_empty() {
        logger::info(&app_handle, "使用 QuietUninstallString 静默卸载");
        installation.quiet_uninstall_string.clone()
    } else if !installation.uninstall_string.is_empty() {
        logger::info(&app_handle, "使用 UninstallString（尝试加静默参数）");
        let raw = installation.uninstall_string.clone();
        // MSI 安装器：MsiExec.exe /I{...} 或 /X{...}
        if raw.to_lowercase().contains("msiexec") {
            // 提取产品 ID 并用 /x 静默卸载
            if let Some(brace_start) = raw.find('{') {
                if let Some(brace_end) = raw[brace_start..].find('}') {
                    let product_id = &raw[brace_start..=brace_start + brace_end];
                    format!("msiexec.exe /x {} /quiet /norestart", product_id)
                } else {
                    raw
                }
            } else {
                raw
            }
        } else {
            // .exe 安装器：追加 /S 静默参数（JetBrains NSIS 卸载器支持 /S）
            format!("{} /S", raw.trim_end_matches(' '))
        }
    } else {
        logger::warn(
            &app_handle,
            &format!("{} 无卸载字符串，跳过卸载器调用（仅清理残留）", installation.product_name),
        );
        return Ok(());
    };

    // 通过 cmd /c 执行卸载命令（处理含空格路径和引号）
    let mut env_vars = HashMap::new();
    env_vars.insert("UNINSTALL_CMD".to_string(), uninstall_cmd);
    let script = r#"
        $cmd = $env:UNINSTALL_CMD
        # 以分离进程启动卸载器并等待完成
        Start-Process cmd.exe -ArgumentList "/c", $cmd -Wait -NoNewWindow
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 600).await {
        Ok(out) => {
            if out.exit_code == 0 {
                logger::info(&app_handle, "卸载器执行完成");
            } else {
                logger::warn(
                    &app_handle,
                    &format!("卸载器返回非零退出码: {}（stderr: {}）", out.exit_code, out.stderr.trim()),
                );
            }
        }
        Err(e) => {
            logger::warn(&app_handle, &format!("卸载器执行异常: {}（继续清理残留）", e));
        }
    }

    logger::info(&app_handle, &format!("{} 卸载流程完成", installation.product_name));
    Ok(())
}
