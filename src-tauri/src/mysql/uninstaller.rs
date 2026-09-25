use super::detector;
use super::super::{detector_base, logger, process_manager, service_manager};
use std::collections::HashMap;
use tauri::AppHandle;

pub async fn stop_mysql_services(app_handle: &AppHandle, services: Vec<String>) -> Result<(), String> {
    for service in &services {
        process_manager::validate_service_name(service)?;
        if let Err(e) = service_manager::stop_service(app_handle, "MySQL", service).await {
            logger::warn(app_handle, &format!("停止服务 {} 失败: {}", service, e));
        }
    }

    Ok(())
}

pub async fn remove_mysql_services(app_handle: &AppHandle, services: Vec<String>) -> Result<(), String> {
    for service in &services {
        process_manager::validate_service_name(service)?;
        match service_manager::delete_service(service).await {
            Ok(()) => logger::info(app_handle, &format!("服务 {} 已删除", service)),
            Err(e) => logger::warn(app_handle, &format!("删除服务 {} 失败: {}", service, e)),
        }
    }

    Ok(())
}

/// 为指定实例构建卸载产品名过滤正则。
///
/// 卸载器（注册表 Uninstall 项 / Get-Package）只会卸载 DisplayName 同时满足
/// "包含 MySQL|MariaDB" 且 "命中本实例过滤正则" 的产品，避免误卸同机其他实例。
/// 过滤规则来源：
/// 1. 安装目录名（如 "MySQL Server 8.0" / "MariaDB 10.6"）
/// 2. 版本号派生的主.次版本（如 8.0.36 -> MySQL\s+Server\s+8\.0 / MariaDB\s+10\.6）
fn build_uninstall_filter(bin_dir: &str, version: &str) -> Option<String> {
    let mut patterns: Vec<String> = Vec::new();

    let folder = std::path::Path::new(bin_dir)
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| {
            let lower = n.to_lowercase();
            lower.contains("mysql") || lower.contains("mariadb")
        });
    if let Some(f) = folder {
        patterns.push(regex::escape(&f));
    }

    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
        let major_minor = regex::escape(&format!("{}.{}", parts[0], parts[1]));
        patterns.push(format!(r"MySQL\s+Server\s+{}", major_minor));
        patterns.push(format!(r"MariaDB\s+{}", major_minor));
    }

    if patterns.is_empty() {
        None
    } else {
        Some(patterns.join("|"))
    }
}

/// 解析实例版本：优先使用传入版本，未知时尝试执行 mysql --version
async fn resolve_instance_version(bin_dir: &str, known_version: &str) -> String {
    if !known_version.is_empty() && known_version != detector_base::VERSION_UNKNOWN && known_version != detector_base::ARCH_UNKNOWN {
        return known_version.to_string();
    }
    let mysql_exe = std::path::Path::new(bin_dir).join("mysql.exe");
    if mysql_exe.exists() {
        if let Ok(output) =
            process_manager::execute_command(mysql_exe.to_str().unwrap_or(""), &["--version"]).await
        {
            if output.exit_code == 0 {
                if let Some(v) = detector::parse_version(&output.stdout) {
                    return v;
                }
            }
        }
    }
    known_version.to_string()
}

/// 策略 1：PowerShell Get-Package（按实例过滤正则限定范围）
async fn uninstall_via_powershell_packages(
    app_handle: &AppHandle,
    filter: &str,
) {
    logger::info(app_handle, "策略 1: PowerShell Get-Package...");
    let mut env_vars = HashMap::new();
    env_vars.insert("UNINSTALL_FILTER".to_string(), filter.to_string());

    let script = r#"
        $ErrorActionPreference = 'Continue'
        $filter = $env:UNINSTALL_FILTER
        if (-not $filter) { Write-Output 'NO_FILTER'; exit 0 }
        Get-Package -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match 'MySQL|MariaDB' -and $_.Name -match $filter } |
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

/// 策略 2：注册表 Uninstall 项（按实例过滤正则限定范围，优先使用 QuietUninstallString）
async fn uninstall_via_registry(app_handle: &AppHandle, filter: &str) {
    logger::info(app_handle, "策略 2: 注册表卸载项...");
    let mut env_vars = HashMap::new();
    env_vars.insert("UNINSTALL_FILTER".to_string(), filter.to_string());

    // 安全说明：通过环境变量传递过滤条件（$env:UNINSTALL_FILTER），
    // PowerShell 脚本内部使用 -match 操作符匹配，不涉及字符串拼接命令。
    // 注册表值（UninstallString/QuietUninstallString）来自系统注册表，
    // 由 Windows 安装程序写入，非用户可控输入。但仍需验证：
    // - 检查路径是否包含系统目录外的可执行文件
    // - 阻止包含 shell 元字符的值
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
                Where-Object { $_.DisplayName -match 'MySQL|MariaDB' -and $_.DisplayName -match $filter } |
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
                                Start-Process cmd.exe -ArgumentList "/c `"$cmd`" /S" -Wait -NoNewWindow
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

/// 卸载选中的 MySQL 实例。
///
/// 安全语义：仅停止/删除选中的服务，并仅卸载与这些实例匹配的 MySQL/MariaDB 产品，
/// 不会触碰同机其他版本/实例（无过滤条件时跳过包卸载并给出提示）。
pub async fn uninstall_selected_mysql(
    app_handle: AppHandle,
    services: Vec<String>,
    instances: Vec<super::super::types::MySQLInstance>,
) -> Result<(), String> {
    logger::info(&app_handle, "开始卸载选中的 MySQL 实例...");

    if services.is_empty() {
        return Err("未选择要卸载的实例服务，操作已取消".to_string());
    }

    logger::info(&app_handle, &format!("将卸载 {} 个选中实例", services.len()));

    // 1. 先停止选中的服务（卸载器需要文件未被占用）
    let _ = stop_mysql_services(&app_handle, services.clone()).await;

    // 2. 逐实例卸载匹配的产品包
    for service in &services {
        let bin_dir = crate::service_manager::get_service_binary_path(service).await;
        let known_version = instances
            .iter()
            .find(|i| i.service_name.as_deref() == Some(service.as_str()))
            .map(|i| i.version.clone())
            .unwrap_or_default();

        let (bin_dir, version) = match bin_dir {
            Some(dir) => {
                let v = resolve_instance_version(&dir, &known_version).await;
                (dir, v)
            }
            None => {
                logger::warn(
                    &app_handle,
                    &format!("无法解析服务 {} 的安装路径，跳过该实例的包卸载（仅删除服务）", service),
                );
                continue;
            }
        };

        match build_uninstall_filter(&bin_dir, &version) {
            Some(filter) => {
                logger::info(
                    &app_handle,
                    &format!("实例 {} (版本 {}) 卸载过滤条件: {}", service, version, filter),
                );
                uninstall_via_powershell_packages(&app_handle, &filter).await;
                uninstall_via_registry(&app_handle, &filter).await;
            }
            None => {
                logger::warn(
                    &app_handle,
                    &format!(
                        "无法为实例 {} 构建产品匹配条件，跳过包卸载以避免误卸其他实例（服务仍会被删除）",
                        service
                    ),
                );
            }
        }
    }

    // 3. 最后删除服务项（MSI 卸载器可能已自行删除，此处幂态兜底）
    let _ = remove_mysql_services(&app_handle, services).await;

    logger::info(&app_handle, "选中实例卸载流程完成，请查看结果");
    Ok(())
}
