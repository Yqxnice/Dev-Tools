use super::detector;
use super::super::{detector_base, logger, process_manager, service_manager};
use std::collections::HashMap;
use tauri::AppHandle;

pub async fn stop_postgresql_services(app_handle: &AppHandle, services: Vec<String>) -> Result<(), String> {
    for service in &services {
        process_manager::validate_service_name(service)?;
        if let Err(e) = service_manager::stop_service(app_handle, "PostgreSQL", service).await {
            logger::warn(app_handle, &format!("停止服务 {} 失败: {}", service, e));
        }
    }
    Ok(())
}

pub async fn remove_postgresql_services(app_handle: &AppHandle, services: Vec<String>) -> Result<(), String> {
    for service in &services {
        process_manager::validate_service_name(service)?;
        match service_manager::delete_service(service).await {
            Ok(()) => logger::info(app_handle, &format!("服务 {} 已删除", service)),
            Err(e) => logger::warn(app_handle, &format!("删除服务 {} 失败: {}", service, e)),
        }
    }
    Ok(())
}

/// 为 PostgreSQL 实例构建卸载过滤正则
/// PostgreSQL 的 DisplayName 形如 "PostgreSQL 16" 或 "PostgreSQL 16 (x64)"
fn build_uninstall_filter(version: &str) -> Option<String> {
    if version.is_empty() || version == detector_base::VERSION_UNKNOWN || version == detector_base::ARCH_UNKNOWN {
        return None;
    }
    // 提取主版本号
    let major = version.split('.').next()?;
    if !major.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!(r"PostgreSQL\s+{}", regex::escape(major)))
}

/// 策略 1：PowerShell Get-Package
async fn uninstall_via_powershell_packages(app_handle: &AppHandle, filter: &str) {
    logger::info(app_handle, "策略 1: PowerShell Get-Package...");
    let mut env_vars = HashMap::new();
    env_vars.insert("UNINSTALL_FILTER".to_string(), filter.to_string());
    let script = r#"
        $ErrorActionPreference = 'Continue'
        $filter = $env:UNINSTALL_FILTER
        if (-not $filter) { Write-Output 'NO_FILTER'; exit 0 }
        Get-Package -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match 'PostgreSQL' -and $_.Name -match $filter } |
            ForEach-Object {
                Write-Output "UNINSTALLING: $($_.Name)"
                Uninstall-Package -Name $_.Name -Force -ErrorAction Continue
            }
    "#;
    match process_manager::execute_powershell_env(script, &env_vars, 300).await {
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

/// 策略 2：注册表 Uninstall 项
async fn uninstall_via_registry(app_handle: &AppHandle, filter: &str) {
    logger::info(app_handle, "策略 2: 注册表卸载项...");
    let mut env_vars = HashMap::new();
    env_vars.insert("UNINSTALL_FILTER".to_string(), filter.to_string());
    // 安全说明：通过环境变量传递过滤条件（$env:UNINSTALL_FILTER），
    // PowerShell 脚本内部使用 -match 操作符匹配，不涉及字符串拼接命令。
    // 注册表值（UninstallString/QuietUninstallString）来自系统注册表，
    // 由 Windows 安装程序写入，非用户可控输入。但仍需验证不包含危险字符。
    let script = r#"
        $ErrorActionPreference = 'Continue'
        $filter = $env:UNINSTALL_FILTER
        if (-not $filter) { Write-Output 'NO_FILTER'; exit 0 }
        $paths = @(
            'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
            'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
        )
        foreach ($path in $paths) {
            Get-ItemProperty $path -ErrorAction SilentlyContinue |
                Where-Object { $_.DisplayName -match 'PostgreSQL' -and $_.DisplayName -match $filter } |
                ForEach-Object {
                    Write-Output "FOUND: $($_.DisplayName)"
                    # 安全检查：验证卸载字符串不包含危险字符
                    $uninstallCmd = if ($_.QuietUninstallString) { $_.QuietUninstallString } elseif ($_.UninstallString) { $_.UninstallString } else { $null }
                    if ($uninstallCmd -and $uninstallCmd -notmatch '[;|&`$]') {
                        if ($_.QuietUninstallString) {
                            Start-Process cmd.exe -ArgumentList "/c `"$($_.QuietUninstallString)`"" -Wait -NoNewWindow
                        } elseif ($_.UninstallString) {
                            $cmd = $_.UninstallString
                            if ($cmd -match 'msiexec') {
                                Start-Process cmd.exe -ArgumentList "/c $cmd /quiet /norestart" -Wait -NoNewWindow
                            } else {
                                Start-Process cmd.exe -ArgumentList "/c `"$cmd`" --mode unattended" -Wait -NoNewWindow
                            }
                        }
                    } else {
                        Write-Output "SKIPPED: 卸载字符串包含危险字符: $uninstallCmd"
                    }
                }
        }
    "#;
    match process_manager::execute_powershell_env(script, &env_vars, 300).await {
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

/// 卸载选中的 PostgreSQL 实例
pub async fn uninstall_selected_postgresql(
    app_handle: AppHandle,
    services: Vec<String>,
    instances: Vec<super::super::types::PostgresqlInstance>,
) -> Result<(), String> {
    logger::info(&app_handle, "开始卸载选中的 PostgreSQL 实例...");

    if services.is_empty() {
        return Err("未选择要卸载的实例服务，操作已取消".to_string());
    }

    logger::info(&app_handle, &format!("将卸载 {} 个选中实例", services.len()));

    // 1. 先停止选中的服务
    let _ = stop_postgresql_services(&app_handle, services.clone()).await;

    // 2. 逐实例卸载匹配的产品包
    for service in &services {
        let bin_dir = crate::service_manager::get_service_binary_path(service).await;
        let known_version = instances
            .iter()
            .find(|i| i.service_name.as_deref() == Some(service.as_str()))
            .map(|i| i.version.clone())
            .unwrap_or_default();

        let (_bin_dir, version) = match bin_dir {
            Some(dir) => {
                // 如果版本未知，从服务名提取主版本号
                let v = if known_version.is_empty() || known_version == detector_base::VERSION_UNKNOWN || known_version == detector_base::ARCH_UNKNOWN {
                    crate::service_manager::get_service_binary_path(service).await
                        .and_then(|_| {
                            // 尝试从服务名提取：postgresql-x64-16 → "16"
                            service.split('-').last().filter(|s| s.chars().all(|c| c.is_ascii_digit())).map(|s| s.to_string())
                        })
                        .unwrap_or(known_version)
                } else {
                    known_version
                };
                (dir, v)
            }
            None => {
                // 即使没有 bin_dir，也尝试从服务名提取版本号来过滤卸载
                let v = service.split('-').last()
                    .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if v.is_empty() {
                    logger::warn(&app_handle, &format!("无法解析服务 {} 的版本，跳过包卸载（仅删除服务）", service));
                    continue;
                }
                (String::new(), v)
            }
        };

        match build_uninstall_filter(&version) {
            Some(filter) => {
                logger::info(&app_handle, &format!("实例 {} (版本 {}) 卸载过滤条件: {}", service, version, filter));
                uninstall_via_powershell_packages(&app_handle, &filter).await;
                uninstall_via_registry(&app_handle, &filter).await;
            }
            None => {
                logger::warn(&app_handle, &format!("无法为实例 {} 构建产品匹配条件，跳过包卸载以避免误卸其他实例（服务仍会被删除）", service));
            }
        }
    }

    // 3. 删除服务项（幂态兜底）
    let _ = remove_postgresql_services(&app_handle, services).await;

    // 4. 终止残留进程
    for inst in &instances {
        if !inst.path.is_empty() {
            let _ = detector::kill_postgresql_processes_in_dir(&inst.path).await;
            detector::wait_postgresql_processes_gone(&app_handle, &inst.path).await;
        }
    }

    logger::info(&app_handle, "选中实例卸载流程完成，请查看结果");
    Ok(())
}
