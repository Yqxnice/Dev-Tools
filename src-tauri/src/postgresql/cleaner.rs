use super::uninstaller;
use super::super::{logger, process_manager, types::PostgresqlInstance};
use super::super::types::{CleanOptions, CleanResult, CleanScanResult, ScannedPath};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const EXCLUDED_NOTE: &str = "仅清理所选实例相关残留；不清理其他版本实例及自定义 data_directory";

#[derive(Debug, Clone)]
pub struct InstanceTargets {
    pub major_version: String,
    pub install_dir: Option<String>,
    pub data_dir: Option<String>,
    pub service_name: Option<String>,
}

pub fn derive_major_version(version: &str) -> String {
    version.split('.').next().unwrap_or(version).to_string()
}

pub fn build_instance_targets(instance: &PostgresqlInstance) -> InstanceTargets {
    let major_version = derive_major_version(&instance.version);

    // install_dir: bin 目录的父目录
    let install_dir = if !instance.path.is_empty() {
        let path = PathBuf::from(&instance.path);
        if path.ends_with("bin") {
            path.parent().map(|p| p.to_string_lossy().to_string())
        } else if path.is_dir() {
            Some(instance.path.clone())
        } else {
            path.parent().map(|p| p.to_string_lossy().to_string())
        }
    } else {
        None
    };

    InstanceTargets {
        major_version,
        install_dir,
        data_dir: instance.data_dir.clone(),
        service_name: instance.service_name.clone(),
    }
}

fn directories_for_instance(targets: &InstanceTargets, options: &CleanOptions) -> Vec<ScannedPath> {
    let mut dirs = Vec::new();

    if options.clean_install_dir {
        if let Some(path) = &targets.install_dir {
            dirs.push(ScannedPath {
                path: path.clone(),
                category: "install_dir".to_string(),
                exists: Path::new(path).exists(),
            });
        }
    }

    if options.clean_program_data {
        if let Some(path) = &targets.data_dir {
            dirs.push(ScannedPath {
                path: path.clone(),
                category: "program_data".to_string(),
                exists: Path::new(path).exists(),
            });
        }
    }

    dirs
}

async fn scan_registry_keys_for_instance(
    targets: &InstanceTargets,
    options: &CleanOptions,
) -> Vec<String> {
    let mut keys = Vec::new();

    if options.clean_registry_services {
        if let Some(svc) = &targets.service_name {
            let key = format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{}", svc);
            if registry_key_exists(&key).await {
                keys.push(key);
            }
        }
    }

    // PostgreSQL 注册表项：HKLM\SOFTWARE\PostgreSQL\Installations\*
    if options.clean_registry_mysql_ab {
        let mut env_vars = HashMap::new();
        env_vars.insert("MAJOR_VER".into(), targets.major_version.clone());
        let script = r#"
            $ver = $env:MAJOR_VER
            $roots = @(
                'HKLM:\SOFTWARE\PostgreSQL\Installations',
                'HKLM:\SOFTWARE\WOW6432Node\PostgreSQL\Installations'
            )
            foreach ($root in $roots) {
                if (Test-Path $root) {
                    Get-ChildItem $root -ErrorAction SilentlyContinue | Where-Object {
                        $branch = $_.GetValue('Branch', '')
                        $major = $_.GetValue('Major Version', '')
                        $disp = $_.GetValue('Display Name', '')
                        ($major -eq $ver) -or ($branch -like "*$ver*") -or ($disp -match "PostgreSQL.*$ver")
                    } | ForEach-Object {
                        $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::', '' -replace '^HKEY_LOCAL_MACHINE', 'HKLM'
                    }
                }
            }
        "#;
        keys.extend(run_powershell_lines_with_env(script, &env_vars).await);
    }

    if options.clean_registry_uninstall {
        let mut env_vars = HashMap::new();
        env_vars.insert("MAJOR_VER".into(), targets.major_version.clone());
        let script = r#"
            $ver = $env:MAJOR_VER
            $roots = @(
                'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
                'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
            )
            foreach ($root in $roots) {
                Get-ChildItem $root -ErrorAction SilentlyContinue | ForEach-Object {
                    $item = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
                    if ($item.DisplayName -and $item.DisplayName -match "PostgreSQL" -and $item.DisplayName -match $ver) {
                        $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::', '' -replace '^HKEY_LOCAL_MACHINE', 'HKLM'
                    }
                }
            }
        "#;
        keys.extend(run_powershell_lines_with_env(script, &env_vars).await);
    }

    if options.clean_user_registry {
        let key = r"HKCU\SOFTWARE\PostgreSQL";
        if registry_key_exists(key).await {
            keys.push(key.to_string());
        }
    }

    keys.sort();
    keys.dedup();
    keys
}

async fn run_powershell_with_env(script: &str, env_vars: &HashMap<String, String>) -> String {
    let mut command = tokio::process::Command::new("powershell");
    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    for (key, value) in env_vars {
        command.env(key, value);
    }
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000);
    }
    match tokio::time::timeout(std::time::Duration::from_secs(30), command.output()).await {
        Ok(Ok(output)) => String::from_utf8_lossy(&output.stdout).to_string(),
        Ok(Err(_)) => String::new(),
        Err(_) => String::new(),
    }
}

async fn run_powershell_lines_with_env(script: &str, env_vars: &HashMap<String, String>) -> Vec<String> {
    let output = run_powershell_with_env(script, env_vars).await;
    output.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()
}

async fn scan_path_entries_for_instance(targets: &InstanceTargets) -> Vec<String> {
    let install = targets.install_dir.as_deref().unwrap_or("").to_string();
    let data = targets.data_dir.as_deref().unwrap_or("").to_string();
    if install.is_empty() && data.is_empty() {
        return Vec::new();
    }
    let mut env_vars = HashMap::new();
    env_vars.insert("INSTALL".into(), install);
    env_vars.insert("DATA".into(), data);
    let script = r#"
        $install = $env:INSTALL
        $data = $env:DATA
        $results = @()
        foreach ($scope in @('Machine', 'User')) {
            $path = [Environment]::GetEnvironmentVariable('Path', $scope)
            if ($path) {
                $path -split ';' | Where-Object {
                    $_ -and (
                        ($install -and $_ -like "*$install*") -or
                        ($data -and $_ -like "*$data*")
                    )
                } | ForEach-Object { "[$scope] $_" }
            }
        }
    "#;
    run_powershell_lines_with_env(script, &env_vars).await
}

async fn scan_start_menu_shortcuts(targets: &InstanceTargets) -> Vec<String> {
    let mut env_vars = HashMap::new();
    env_vars.insert("MAJOR_VER".into(), targets.major_version.clone());
    let script = r#"
        $ver = $env:MAJOR_VER
        $paths = @(
            [Environment]::GetFolderPath('CommonPrograms'),
            [Environment]::GetFolderPath('Programs')
        )
        $shortcuts = @()
        foreach ($basePath in $paths) {
            if (-not (Test-Path $basePath)) { continue }
            Get-ChildItem -Path $basePath -Directory -ErrorAction SilentlyContinue | Where-Object {
                $_.Name -match 'PostgreSQL' -and $_.Name -match $ver
            } | ForEach-Object {
                $shortcuts += $_.FullName
            }
        }
        $shortcuts
    "#;
    run_powershell_lines_with_env(script, &env_vars).await
}

async fn registry_key_exists(key: &str) -> bool {
    process_manager::execute_command("reg", &["query", key])
        .await
        .map(|output| output.exit_code == 0)
        .unwrap_or(false)
}

fn preview_scan_options() -> CleanOptions {
    CleanOptions {
        clean_user_registry: true,
        ..CleanOptions::default()
    }
}

pub async fn scan_postgresql_residuals(
    app_handle: AppHandle,
    selected_instance: PostgresqlInstance,
) -> CleanScanResult {
    let targets = build_instance_targets(&selected_instance);
    logger::info(&app_handle, &format!("开始扫描实例残留: PostgreSQL {}", targets.major_version));

    let mut services: Vec<String> = Vec::new();
    if let Some(svc_name) = &targets.service_name {
        let status = crate::service_manager::check_service_status(svc_name).await;
        if status != "未安装" {
            services.push(svc_name.clone());
        }
    }

    let directories = directories_for_instance(&targets, &preview_scan_options());
    let registry_keys = scan_registry_keys_for_instance(&targets, &preview_scan_options()).await;
    let start_menu_shortcuts = scan_start_menu_shortcuts(&targets).await;
    let path_entries = scan_path_entries_for_instance(&targets).await;

    logger::info(&app_handle, &format!(
        "扫描完成 [PostgreSQL {}]: {} 个服务, {} 个目录, {} 个注册表项, {} 个开始菜单项, {} 条 PATH",
        targets.major_version,
        services.len(),
        directories.iter().filter(|d| d.exists).count(),
        registry_keys.len(),
        start_menu_shortcuts.len(),
        path_entries.len()
    ));

    CleanScanResult {
        instance_label: format!("PostgreSQL {}", targets.major_version),
        selected_version: selected_instance.version.clone(),
        services,
        directories,
        registry_keys,
        start_menu_shortcuts,
        path_entries,
        excluded_note: EXCLUDED_NOTE.to_string(),
    }
}

fn instance_bin_dir(targets: &InstanceTargets) -> Option<String> {
    let root = targets.install_dir.as_deref()?;
    let bin_sub = Path::new(root).join("bin");
    if bin_sub.is_dir() {
        Some(bin_sub.to_string_lossy().to_string())
    } else {
        Some(root.to_string())
    }
}

async fn kill_instance_processes(app_handle: &AppHandle, result: &mut CleanResult, targets: &InstanceTargets) {
    let bin_dir = match instance_bin_dir(targets) {
        Some(dir) => dir,
        None => {
            logger::warn(app_handle, "无法解析实例安装目录，跳过进程终止");
            return;
        }
    };

    if let Some(svc) = &targets.service_name {
        logger::info(app_handle, &format!("终止实例服务进程: {}", svc));
        if let Ok(output) = process_manager::execute_command("sc", &["queryex", svc]).await {
            if output.exit_code == 0 {
                for line in output.stdout.lines() {
                    let trimmed = line.trim();
                    if let Some(pid_str) = trimmed.strip_prefix("PID") {
                        let pid = pid_str.trim().trim_start_matches(':').trim();
                        if !pid.is_empty() && pid != "0" {
                            match process_manager::execute_command("taskkill", &["/F", "/PID", pid]).await {
                                Ok(out) if out.exit_code == 0 => {
                                    result.cleaned_items.push(format!("已终止实例进程 PID {} (服务 {})", pid, svc));
                                }
                                Ok(out) => {
                                    result.errors.push(format!("终止 PID {} 失败: {}", pid, out.stderr.trim()));
                                }
                                Err(e) => result.errors.push(format!("终止 PID {} 异常: {}", pid, e)),
                            }
                        }
                    }
                }
            }
        }
    }

    logger::info(app_handle, &format!("终止实例目录下的 PostgreSQL 进程: {}", bin_dir));
    match crate::postgresql::detector::kill_postgresql_processes_in_dir(&bin_dir).await {
        Ok(n) if n > 0 => logger::info(app_handle, &format!("已终止 {} 个实例进程", n)),
        Ok(_) => {}
        Err(e) => result.errors.push(format!("终止实例进程异常: {}", e)),
    }

    crate::postgresql::detector::wait_postgresql_processes_gone(app_handle, &bin_dir).await;
}

async fn remove_residual_services(app_handle: &AppHandle, result: &mut CleanResult, services: Vec<String>) {
    if services.is_empty() {
        logger::info(app_handle, "所选实例没有关联服务，跳过服务删除");
        return;
    }
    if let Err(e) = uninstaller::stop_postgresql_services(app_handle, services.clone()).await {
        result.errors.push(format!("停止服务异常: {}", e));
    }
    if let Err(e) = uninstaller::remove_postgresql_services(app_handle, services.clone()).await {
        result.errors.push(format!("删除服务异常: {}", e));
    } else {
        for service in services {
            result.cleaned_items.push(format!("已删除服务: {}", service));
        }
    }
}

async fn remove_instance_directories(
    app_handle: &AppHandle,
    result: &mut CleanResult,
    targets: &InstanceTargets,
    options: &CleanOptions,
) {
    for dir in directories_for_instance(targets, options) {
        if !dir.exists {
            logger::info(app_handle, &format!("目录不存在，跳过: {}", dir.path));
            continue;
        }
        match tokio::fs::remove_dir_all(&dir.path).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除目录 [{}]: {}", dir.category, dir.path));
                logger::info(app_handle, &format!("已删除: {}", dir.path));
            }
            Err(e) => {
                result.errors.push(format!("无法删除 {}: {}", dir.path, e));
                logger::warn(app_handle, &format!("删除失败 {}: {}", dir.path, e));
            }
        }
    }
}

async fn delete_registry_key(_app_handle: &AppHandle, key: &str) -> Result<(), String> {
    let reg_result = process_manager::execute_command("reg", &["delete", key, "/f"]).await;
    if let Ok(output) = reg_result {
        if output.exit_code == 0 {
            return Ok(());
        }
    }
    let key_normalized = if key.starts_with("HKLM") { key.replace("HKLM", "HKLM:") }
        else if key.starts_with("HKCU") { key.replace("HKCU", "HKCU:") }
        else { key.to_string() };
    let mut env_vars = HashMap::new();
    env_vars.insert("REG_KEY".into(), key_normalized);
    let script = r#"
        try {
            Remove-Item -Path $env:REG_KEY -Recurse -Force -ErrorAction Stop
            Write-Output 'SUCCESS'
        } catch { Write-Output "ERROR: $($_.Exception.Message)" }
    "#;
    let ps_output = run_powershell_with_env(script, &env_vars).await;
    if ps_output.trim() == "SUCCESS" { Ok(()) }
    else { Err(format!("PowerShell 删除失败: {}", ps_output.trim())) }
}

async fn clean_registry(app_handle: &AppHandle, result: &mut CleanResult, targets: &InstanceTargets, options: &CleanOptions) {
    let keys = scan_registry_keys_for_instance(targets, options).await;
    if keys.is_empty() {
        logger::info(app_handle, "未发现需要清理的注册表项");
        return;
    }
    for key in keys {
        match delete_registry_key(app_handle, &key).await {
            Ok(_) => result.cleaned_items.push(format!("删除注册表项: {}", key)),
            Err(e) => {
                if !e.to_lowercase().contains("找不到") && !e.to_lowercase().contains("不存在") && !e.to_lowercase().contains("not found") && !e.to_lowercase().contains("does not exist") {
                    result.errors.push(format!("删除注册表 {} 失败: {}", key, e));
                }
            }
        }
    }
}

async fn clean_start_menu_shortcuts(app_handle: &AppHandle, result: &mut CleanResult, targets: &InstanceTargets) {
    let shortcuts = scan_start_menu_shortcuts(targets).await;
    if shortcuts.is_empty() {
        logger::info(app_handle, "未发现需要清理的开始菜单快捷方式");
        return;
    }
    for path in shortcuts {
        match tokio::fs::remove_dir_all(&path).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除开始菜单快捷方式目录: {}", path));
                logger::info(app_handle, &format!("已删除开始菜单快捷方式: {}", path));
            }
            Err(e) => result.errors.push(format!("无法删除开始菜单快捷方式目录 {}: {}", path, e)),
        }
    }
}

async fn clean_path_entries_for_instance(_app_handle: &AppHandle, result: &mut CleanResult, targets: &InstanceTargets) {
    let install = targets.install_dir.as_deref().unwrap_or("").to_string();
    let data = targets.data_dir.as_deref().unwrap_or("").to_string();
    let mut env_vars = HashMap::new();
    env_vars.insert("INSTALL".into(), install);
    env_vars.insert("DATA".into(), data);
    let script = r#"
        $install = $env:INSTALL
        $data = $env:DATA
        $changed = @()
        foreach ($scope in @('Machine', 'User')) {
            $path = [Environment]::GetEnvironmentVariable('Path', $scope)
            if (-not $path) { continue }
            $parts = $path -split ';' | Where-Object {
                $_ -and -not (
                    ($install -and $_ -like "*$install*") -or
                    ($data -and $_ -like "*$data*")
                )
            }
            $newPath = ($parts -join ';').TrimEnd(';')
            if ($newPath -ne $path) {
                [Environment]::SetEnvironmentVariable('Path', $newPath, $scope)
                $changed += $scope
            }
        }
        if ($changed.Count -gt 0) { 'UPDATED:' + ($changed -join ',') } else { 'NONE' }
    "#;
    let output = run_powershell_with_env(script, &env_vars).await;
    let stdout = output.trim();
    if stdout.starts_with("UPDATED:") {
        result.cleaned_items.push(format!("已清理实例相关 PATH: {}", stdout.trim_start_matches("UPDATED:")));
    }
}

pub async fn clean_postgresql_residuals(
    app_handle: AppHandle,
    selected_instance: PostgresqlInstance,
    options: CleanOptions,
) -> CleanResult {
    let targets = build_instance_targets(&selected_instance);
    let mut result = CleanResult {
        success: true,
        message: format!("PostgreSQL {} 残留清理完成", targets.major_version),
        cleaned_items: Vec::new(),
        errors: Vec::new(),
    };

    logger::info(&app_handle, &format!("开始清理实例残留: PostgreSQL {}", targets.major_version));
    logger::info(&app_handle, EXCLUDED_NOTE);

    if options.kill_processes {
        kill_instance_processes(&app_handle, &mut result, &targets).await;
    }

    if options.remove_services {
        let services: Vec<String> = targets.service_name.clone().into_iter().collect();
        remove_residual_services(&app_handle, &mut result, services).await;
    }

    if options.kill_processes {
        if let Some(bin_dir) = instance_bin_dir(&targets) {
            logger::info(&app_handle, "再次检查并终止实例遗留进程...");
            if let Ok(n) = crate::postgresql::detector::kill_postgresql_processes_in_dir(&bin_dir).await {
                if n > 0 { logger::info(&app_handle, &format!("补杀 {} 个遗留进程", n)); }
            }
            crate::postgresql::detector::wait_postgresql_processes_gone(&app_handle, &bin_dir).await;
        }
    }

    if options.clean_registry_uninstall || options.clean_registry_mysql_ab || options.clean_registry_services || options.clean_user_registry {
        clean_registry(&app_handle, &mut result, &targets, &options).await;
    }

    remove_instance_directories(&app_handle, &mut result, &targets, &options).await;

    if options.clean_start_menu {
        clean_start_menu_shortcuts(&app_handle, &mut result, &targets).await;
    }

    if options.clean_path {
        clean_path_entries_for_instance(&app_handle, &mut result, &targets).await;
    }

    if !result.errors.is_empty() {
        result.success = false;
        result.message = format!(
            "PostgreSQL {} 部分清理失败（成功 {} 项，失败 {} 项）",
            targets.major_version,
            result.cleaned_items.len(),
            result.errors.len()
        );
    } else if result.cleaned_items.is_empty() {
        result.message = format!("PostgreSQL {} 未发现需要清理的残留", targets.major_version);
    }

    result
}
