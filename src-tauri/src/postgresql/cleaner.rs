//! PostgreSQL 实例残留扫描与清理：注册表 / PATH / 开始菜单 / 服务 / 数据目录。
//!
//! 动态参数一律通过环境变量注入 PowerShell（`$env:KEY`），
//! 公共执行原语复用 [`crate::db_common`]。

use super::uninstaller;
use super::super::{detector_base, logger, process_manager, types::PostgresqlInstance};
use super::super::types::{CleanOptions, CleanResult, CleanScanResult, ScannedPath};
use crate::db_common::{run_powershell_lines_with_env, run_powershell_with_env};
use crate::delete_safety::{validate_deletable_dir, validate_registry_key, DeletionRules};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const EXCLUDED_NOTE: &str = "仅清理所选实例相关残留；不清理其他版本实例及自定义 data_directory";

/// 允许删除其**子键**的注册表根（键必须严格位于其下）
const ALLOWED_REGISTRY_PREFIXES: &[&str] = &[
    r"HKLM\SYSTEM\CurrentControlSet\Services",
    r"HKLM\SOFTWARE\PostgreSQL",
    r"HKLM\SOFTWARE\WOW6432Node\PostgreSQL",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
];

/// 允许**整键删除**的产品根键
const ALLOWED_REGISTRY_EXACT: &[&str] = &[r"HKCU\SOFTWARE\PostgreSQL"];

/// 安装目录删除规则：路径必须能证明属于 PostgreSQL。
fn install_dir_rules() -> DeletionRules {
    DeletionRules::new().keywords(&["postgres", "pgsql", "edb"])
}

/// 数据目录只允许删除**标准安装位置**（`%ProgramData%\PostgreSQL` 等）。
///
/// 与 `EXCLUDED_NOTE` 承诺的"不清理自定义 data_directory"保持一致：
/// 数据目录一旦误删不可恢复，而本工具**不做任何备份**（见 DISCLAIMER.md），
/// 因此自定义位置的一律跳过。
fn allowed_data_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["ProgramData", "PROGRAMDATA"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                roots.push(PathBuf::from(&v).join("PostgreSQL"));
                roots.push(PathBuf::from(&v).join("edb"));
                break;
            }
        }
    }
    roots
}

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
            // 服务名来自前端，先做字符集校验；归属校验在删除服务/进程处进行
            if let Err(e) = process_manager::validate_service_name(svc) {
                eprintln!("[WARN] 跳过非法服务名: {}", e);
            } else {
                let key = format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{}", svc);
                if registry_key_exists(&key).await {
                    keys.push(key);
                }
            }
        }
    }

    // PostgreSQL 注册表项：HKLM\SOFTWARE\PostgreSQL\Installations\*
    if options.clean_registry_mysql_ab {
        let mut env_vars = HashMap::new();
        env_vars.insert("MAJOR_VER".into(), targets.major_version.clone());
        // 空版本号会让 `-like "*"` / `-match ""` 恒真，命中该根键下**所有**版本
        let script = r#"
            $ver = $env:MAJOR_VER
            $hasVer = -not [string]::IsNullOrWhiteSpace($ver)
            if (-not $hasVer) { exit 0 }
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
            if ([string]::IsNullOrWhiteSpace($ver)) { exit 0 }
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
    // 空版本号会让 `-match ""` 恒真，返回 PostgreSQL 下**所有**版本目录
    let script = r#"
        $ver = $env:MAJOR_VER
        if ([string]::IsNullOrWhiteSpace($ver)) { exit 0 }
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

/// 开始菜单删除白名单：必须是 CommonPrograms/Programs 的**直接子目录**，
/// 且目录名以 `PostgreSQL` 开头（脚本扫描出的就是这一层）。
fn validate_start_menu_dir(raw: &str) -> Result<PathBuf, String> {
    if raw.contains("..") || raw.contains('*') || raw.contains('?') {
        return Err("路径含通配符或上级引用".to_string());
    }
    let canonical = Path::new(raw)
        .canonicalize()
        .map_err(|e| format!("无法解析路径: {}", e))?;
    if !canonical.is_dir() {
        return Err("目标不是目录".to_string());
    }

    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["APPDATA", "PROGRAMDATA"] {
        if let Ok(v) = std::env::var(var) {
            bases.push(
                PathBuf::from(v)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
    }
    if bases.is_empty() {
        return Err("无法定位开始菜单目录".to_string());
    }

    let parent = canonical
        .parent()
        .ok_or_else(|| "无法解析父目录".to_string())?;
    if !bases
        .iter()
        .filter_map(|b| b.canonicalize().ok())
        .any(|b| parent == b)
    {
        return Err("不是开始菜单的直接子目录".to_string());
    }

    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if !name.to_lowercase().starts_with("postgresql") {
        return Err(format!("目录名不是 PostgreSQL 相关: {}", name));
    }
    Ok(canonical)
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
        if status != detector_base::STATUS_NOT_INSTALLED {
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
        // 归属校验：service_name 来自前端，仅靠字符集校验不足以阻止
        // 指定任意服务并 taskkill 其 PID
        match uninstaller::assert_postgresql_service(svc).await {
            Ok(None) => {
                logger::info(app_handle, &format!("服务 {} 不存在，跳过进程终止", svc));
            }
            Err(e) => {
                result.errors.push(e.clone());
                logger::warn(app_handle, &e);
            }
            Ok(Some(_)) => {
                logger::info(app_handle, &format!("终止实例服务进程: {}", svc));
                if let Ok(output) = process_manager::execute_command("sc", &["queryex", svc]).await {
                    if output.exit_code == 0 {
                        for line in output.stdout.lines() {
                            let trimmed = line.trim();
                            if let Some(pid_str) = trimmed.strip_prefix("PID") {
                                let pid = pid_str.trim().trim_start_matches(':').trim();
                                if !pid.is_empty() && pid != "0" && pid.chars().all(|c| c.is_ascii_digit()) {
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
        // 安全校验（原实现直接 remove_dir_all，且 data_dir 完全来自前端 IPC）：
        // - install_dir：关键词归属证明
        // - program_data(data_dir)：仅允许标准 ProgramData 位置，自定义 datadir 跳过
        let rules = match dir.category.as_str() {
            "program_data" => {
                let roots = allowed_data_roots();
                if roots.is_empty() {
                    result.errors.push(format!(
                        "无法定位标准数据目录根，已跳过: {}",
                        dir.path
                    ));
                    continue;
                }
                DeletionRules::new().allowed_roots(roots)
            }
            _ => install_dir_rules(),
        };
        let canonical = match validate_deletable_dir(&dir.path, &rules) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("已阻止删除 [{}] {}: {}", dir.category, dir.path, e);
                if dir.category == "program_data" {
                    // 自定义 datadir 属于"按设计跳过"，不算失败
                    logger::info(app_handle, &msg);
                } else {
                    logger::warn(app_handle, &msg);
                    result.errors.push(msg);
                }
                continue;
            }
        };
        let target = canonical.to_string_lossy().to_string();
        match tokio::fs::remove_dir_all(&canonical).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除目录 [{}]: {}", dir.category, target));
                logger::info(app_handle, &format!("已删除: {}", target));
            }
            Err(e) => {
                result.errors.push(format!("无法删除 {}: {}", target, e));
                logger::warn(app_handle, &format!("删除失败 {}: {}", target, e));
            }
        }
    }
}

async fn delete_registry_key(_app_handle: &AppHandle, key: &str) -> Result<(), String> {
    // 删除前白名单校验：拒绝白名单之外的根键及根键本身
    validate_registry_key(key, ALLOWED_REGISTRY_PREFIXES, ALLOWED_REGISTRY_EXACT)?;

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
        if let Err(e) = validate_start_menu_dir(&path) {
            let msg = format!("已阻止删除开始菜单路径 {}: {}", path, e);
            logger::warn(app_handle, &msg);
            result.errors.push(msg);
            continue;
        }
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
