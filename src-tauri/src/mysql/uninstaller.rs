use super::super::{logger, process_manager};
use tauri::AppHandle;

pub async fn stop_mysql_services(app_handle: &AppHandle, services: Vec<String>) -> Result<(), String> {
    for service in services {
        match process_manager::execute_command("net", &["stop", &service]).await {
            Ok(output) => {
                if output.exit_code == 0 {
                    logger::info(app_handle, &format!("服务 {} 已停止", service));
                } else {
                    logger::warn(
                        app_handle,
                        &format!("停止服务 {} 失败: {}", service, output.stderr.trim()),
                    );
                }
            }
            Err(e) => logger::warn(app_handle, &format!("停止服务 {} 异常: {}", service, e)),
        }
    }

    Ok(())
}

pub async fn remove_mysql_services(app_handle: &AppHandle, services: Vec<String>) -> Result<(), String> {
    for service in services {
        match process_manager::execute_command("sc", &["delete", &service]).await {
            Ok(output) => {
                if output.exit_code == 0 {
                    logger::info(app_handle, &format!("服务 {} 已删除", service));
                } else {
                    logger::warn(
                        app_handle,
                        &format!("删除服务 {} 失败: {}", service, output.stderr.trim()),
                    );
                }
            }
            Err(e) => logger::warn(app_handle, &format!("删除服务 {} 异常: {}", service, e)),
        }
    }

    Ok(())
}

async fn uninstall_via_powershell_packages(app_handle: &AppHandle) {
    logger::info(app_handle, "策略 1: PowerShell...");
    let script = r#"
        $ErrorActionPreference = 'Continue'
        Get-Package -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match 'MySQL|MariaDB' } |
            ForEach-Object {
                Write-Output "UNINSTALLING: $($_.Name)"
                Uninstall-Package -Name $_.Name -Force -ErrorAction Continue
            }
    "#;
    match process_manager::execute_command_with_timeout(
        "powershell",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
        180,
    )
    .await
    {
        Ok(output) => {
            if !output.stdout.trim().is_empty() {
                logger::info(app_handle, &output.stdout);
            }
            if output.exit_code != 0 && !output.stderr.trim().is_empty() {
                logger::warn(app_handle, &format!("PowerShell 卸载警告: {}", output.stderr.trim()));
            }
        }
        Err(e) => logger::warn(app_handle, &format!("PowerShell 卸载失败: {}", e)),
    }
}

async fn uninstall_via_registry(app_handle: &AppHandle) {
    logger::info(app_handle, "策略 2: 注册表...");
    let script = r#"
        $ErrorActionPreference = 'Continue'
        $paths = @(
            'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
            'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
        )
        foreach ($path in $paths) {
            Get-ItemProperty $path -ErrorAction SilentlyContinue |
                Where-Object { $_.DisplayName -match 'MySQL|MariaDB' } |
                ForEach-Object {
                    Write-Output "FOUND: $($_.DisplayName)"
                    if ($_.UninstallString) {
                        $cmd = $_.UninstallString
                        if ($cmd -match 'msiexec') {
                            Start-Process cmd.exe -ArgumentList "/c $cmd /quiet /norestart" -Wait -NoNewWindow
                        } else {
                            Start-Process cmd.exe -ArgumentList "/c `"$cmd`" /S" -Wait -NoNewWindow
                        }
                    }
                }
        }
    "#;
    match process_manager::execute_command_with_timeout(
        "powershell",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
        180,
    )
    .await
    {
        Ok(output) => {
            if !output.stdout.trim().is_empty() {
                logger::info(app_handle, &output.stdout);
            }
            if output.exit_code != 0 && !output.stderr.trim().is_empty() {
                logger::warn(app_handle, &format!("注册表卸载警告: {}", output.stderr.trim()));
            }
        }
        Err(e) => logger::warn(app_handle, &format!("注册表卸载失败: {}", e)),
    }
}

async fn uninstall_via_wmic(app_handle: &AppHandle) {
    logger::info(app_handle, "策略 3: wmic...");
    match process_manager::execute_command_with_timeout(
        "wmic",
        &[
            "product",
            "where",
            "name",
            "like",
            "%MySQL%",
            "call",
            "uninstall",
            "/nointeractive",
        ],
        180,
    )
    .await
    {
        Ok(output) => {
            if output.exit_code == 0 {
            } else {
                logger::warn(
                    app_handle,
                    &format!(
                        "wmic 卸载失败 (code={}): {}",
                        output.exit_code,
                        output.stderr.trim()
                    ),
                );
            }
        }
        Err(e) => logger::warn(app_handle, &format!("wmic 不可用或执行失败: {}", e)),
    }
}

pub async fn uninstall_selected_mysql(app_handle: AppHandle, services: Vec<String>) -> Result<(), String> {
    logger::info(&app_handle, "开始卸载 MySQL...");

    if services.is_empty() {
        logger::info(&app_handle, "没有选择要卸载的服务，将尝试卸载所有 MySQL 相关产品...");
    } else {
        logger::info(&app_handle, &format!("将卸载 {} 个服务", services.len()));
        let _ = stop_mysql_services(&app_handle, services.clone()).await;
        let _ = remove_mysql_services(&app_handle, services).await;
    }

    uninstall_via_powershell_packages(&app_handle).await;
    uninstall_via_registry(&app_handle).await;
    uninstall_via_wmic(&app_handle).await;

    logger::info(&app_handle, "卸载完成，请查看结果");

    Ok(())
}
