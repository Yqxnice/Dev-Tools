use super::uninstaller;
use super::super::{logger, process_manager, types::MySQLInstance};
use super::super::types::{CleanOptions, CleanResult, CleanScanResult, ScannedPath};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const EXCLUDED_NOTE: &str =
    "仅清理所选实例相关残留；不清理 XAMPP/WAMP/AppServ、其他版本实例及自定义 datadir";

const EXCLUDED_PATH_PREFIXES: &[&str] = &[
    r"C:\xampp",
    r"C:\wamp",
    r"C:\wamp64",
    r"C:\appserv",
];

#[derive(Debug, Clone)]
pub struct InstanceTargets {
    pub server_folder_name: String,
    pub version_pattern: String,
    pub install_dir: Option<String>,
    pub program_data_dir: Option<String>,
    pub service_name: Option<String>,
}

fn is_excluded_path(path: &str) -> bool {
    let normalized = path.to_lowercase();
    EXCLUDED_PATH_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(&prefix.to_lowercase()))
}

pub fn derive_version_pattern(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}

pub fn derive_server_folder_name(instance: &MySQLInstance) -> String {
    if !instance.path.is_empty() {
        let path = Path::new(&instance.path);
        let mut current = Some(path);
        while let Some(p) = current {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("MySQL Server") || name.starts_with("MariaDB") {
                    return name.to_string();
                }
            }
            current = p.parent();
        }
    }

    if let Some(svc) = &instance.service_name {
        if svc.starts_with("MySQL") && svc.len() > 5 {
            let num_part = &svc[5..];
            if num_part.len() >= 2 {
                let major = &num_part[0..1];
                let minor = &num_part[1..];
                return format!("MySQL Server {}.{}", major, minor);
            }
        }
    }

    let pattern = derive_version_pattern(&instance.version);
    if !pattern.is_empty() && pattern != "未知" {
        format!("MySQL Server {}", pattern)
    } else {
        "MySQL Server".to_string()
    }
}

pub fn build_instance_targets(instance: &MySQLInstance) -> InstanceTargets {
    let server_folder_name = derive_server_folder_name(instance);
    let version_pattern = derive_version_pattern(&instance.version);

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

    let program_data_dir = if server_folder_name.starts_with("MySQL Server")
        || server_folder_name.starts_with("MariaDB")
    {
        Some(
            PathBuf::from(r"C:\ProgramData\MySQL")
                .join(&server_folder_name)
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };

    InstanceTargets {
        server_folder_name,
        version_pattern,
        install_dir,
        program_data_dir,
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
        if let Some(path) = &targets.program_data_dir {
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

    if options.clean_registry_mysql_ab {
        let script = format!(
            r#"
                $folder = '{}'
                $pattern = '{}'
                $roots = @(
                    'HKLM:\SOFTWARE\MySQL AB',
                    'HKLM:\SOFTWARE\WOW6432Node\MySQL AB'
                )
                foreach ($root in $roots) {{
                    if (Test-Path $root) {{
                        if ($root -match 'MySQL AB$') {{
                            Get-ChildItem $root -ErrorAction SilentlyContinue | Where-Object {{
                                $_.PSChildName -match $pattern -or $_.PSChildName -eq $folder
                            }} | ForEach-Object {{
                                $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::', '' -replace '^HKEY_LOCAL_MACHINE', 'HKLM'
                            }}
                            if ($folder -match $pattern) {{
                                $root -replace '^HKLM:', 'HKLM'
                            }}
                        }}
                    }}
                }}
            "#,
            targets.server_folder_name.replace('\'', "''"),
            targets.version_pattern.replace('\'', "''"),
        );
        keys.extend(run_powershell_lines(&script).await);
    }

    if options.clean_registry_uninstall {
        let script = format!(
            r#"
                $folder = '{}'
                $pattern = '{}'
                $roots = @(
                    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
                    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
                )
                foreach ($root in $roots) {{
                    Get-ChildItem $root -ErrorAction SilentlyContinue | ForEach-Object {{
                        $item = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
                        if ($item.DisplayName -and (
                            $item.DisplayName -eq $folder -or
                            $item.DisplayName -match "MySQL.*$pattern" -or
                            $item.DisplayName -match "MariaDB.*$pattern"
                        )) {{
                            $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::', '' -replace '^HKEY_LOCAL_MACHINE', 'HKLM'
                        }}
                    }}
                }}
            "#,
            targets.server_folder_name.replace('\'', "''"),
            targets.version_pattern.replace('\'', "''"),
        );
        keys.extend(run_powershell_lines(&script).await);
    }

    if options.clean_odbc {
        let odbc_pattern = if targets.version_pattern.starts_with('8') {
            "MySQL ODBC 8"
        } else if targets.version_pattern.starts_with('5') {
            "MySQL ODBC 5"
        } else {
            "MySQL ODBC"
        };
        let script = format!(
            r#"
                Get-ChildItem 'HKLM:\SOFTWARE\ODBC\ODBCINST.INI' -ErrorAction SilentlyContinue |
                    Where-Object {{ $_.PSChildName -match '{}' }} |
                    ForEach-Object {{ 'HKLM\SOFTWARE\ODBC\ODBCINST.INI\' + $_.PSChildName }}
            "#,
            odbc_pattern.replace('\'', "''"),
        );
        keys.extend(run_powershell_lines(&script).await);
    }

    if options.clean_user_registry {
        let key = r"HKCU\SOFTWARE\MySQL AB";
        if registry_key_exists(key).await {
            keys.push(key.to_string());
        }
    }

    // 追加扫描 MySQL Installer 产品缓存
    if options.clean_registry_installer {
        let installer_script = format!(
            r#"
$verPattern = '{}'
$paths = @('HKLM:\SOFTWARE\MySQL\Installer\Products','HKLM:\SOFTWARE\WOW6432Node\MySQL\Installer\Products')
foreach($p in $paths){{
    if(Test-Path $p){{
        Get-ChildItem $p -ErrorAction SilentlyContinue | Where-Object {{
            $disp = $_.GetValue('DisplayName','')
            $prodVer = $_.GetValue('Version','')
            ($disp -match "MySQL Server" -and $disp -match $verPattern) -or ($prodVer -like "$verPattern*")
        }} | ForEach-Object {{
            $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::','HKLM'
        }}
    }}
}}
"#,
            targets.version_pattern.replace('\'',"''")
        );
        keys.extend(run_powershell_lines(&installer_script).await);
    }

    keys.sort();
    keys.dedup();
    keys
}

async fn run_powershell_lines(script: &str) -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(output) = process_manager::execute_command(
        "powershell",
        &["-NoProfile", "-Command", script],
    )
    .await
    {
        if output.exit_code == 0 {
            lines.extend(
                output
                    .stdout
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string),
            );
        }
    }
    lines
}

async fn scan_path_entries_for_instance(targets: &InstanceTargets) -> Vec<String> {
    let install = targets
        .install_dir
        .as_deref()
        .unwrap_or("")
        .replace('\\', "\\\\");
    let data = targets
        .program_data_dir
        .as_deref()
        .unwrap_or("")
        .replace('\\', "\\\\");

    if install.is_empty() && data.is_empty() {
        return Vec::new();
    }

    let script = format!(
        r#"
            $install = '{}'
            $data = '{}'
            $results = @()
            foreach ($scope in @('Machine', 'User')) {{
                $path = [Environment]::GetEnvironmentVariable('Path', $scope)
                if ($path) {{
                    $path -split ';' | Where-Object {{
                        $_ -and (
                            ($install -and $_ -like "*$install*") -or
                            ($data -and $_ -like "*$data*")
                        )
                    }} | ForEach-Object {{ "[$scope] $_" }}
                }}
            }}
            $results | Sort-Object -Unique
        "#,
        install.replace('\'', "''"),
        data.replace('\'', "''"),
    );

    run_powershell_lines(&script).await
}

async fn scan_start_menu_shortcuts(targets: &InstanceTargets) -> Vec<String> {
    let script = format!(
        r#"
            $verPattern = '{}'
            $paths = @(
                [Environment]::GetFolderPath('CommonPrograms'),
                [Environment]::GetFolderPath('Programs')
            )
            $shortcuts = @()
            foreach ($basePath in $paths) {{
                if (-not (Test-Path $basePath)) {{ continue }}
                $mysqlDir = Join-Path $basePath 'MySQL'
                if (-not (Test-Path $mysqlDir)) {{ continue }}
                Get-ChildItem -Path $mysqlDir -Directory -ErrorAction SilentlyContinue | Where-Object {{
                    $_.Name -match $verPattern
                }} | ForEach-Object {{
                    $shortcuts += $_.FullName
                }}
            }}
            $shortcuts
        "#,
        targets.version_pattern.replace('\'', "''")
    );
    run_powershell_lines(&script).await
}

async fn registry_key_exists(key: &str) -> bool {
    process_manager::execute_command("reg", &["query", key])
        .await
        .map(|output| output.exit_code == 0)
        .unwrap_or(false)
}

fn preview_scan_options() -> CleanOptions {
    CleanOptions {
        clean_odbc: true,
        clean_user_registry: true,
        ..CleanOptions::default()
    }
}

pub async fn scan_mysql_residuals(
    app_handle: AppHandle,
    selected_instance: MySQLInstance,
) -> CleanScanResult {
    let targets = build_instance_targets(&selected_instance);
    logger::info(
        &app_handle,
        &format!(
            "开始扫描实例残留: {} ({})",
            targets.server_folder_name, targets.version_pattern
        ),
    );

    let mut services: Vec<String> = Vec::new();
    if let Some(svc_name) = &targets.service_name {
        let status = crate::mysql::detector::check_service_status(svc_name).await;
        if status != "未安装" {
            services.push(svc_name.clone());
        }
    }

    let directories: Vec<ScannedPath> = directories_for_instance(&targets, &preview_scan_options());
    let registry_keys = scan_registry_keys_for_instance(&targets, &preview_scan_options()).await;
    let start_menu_shortcuts = scan_start_menu_shortcuts(&targets).await;
    let path_entries = scan_path_entries_for_instance(&targets).await;

    logger::info(
        &app_handle,
        &format!(
            "扫描完成 [{}]: {} 个服务, {} 个目录, {} 个注册表项, {} 个开始菜单项, {} 条 PATH",
            targets.server_folder_name,
            services.len(),
            directories.iter().filter(|d| d.exists).count(),
            registry_keys.len(),
            start_menu_shortcuts.len(),
            path_entries.len()
        ),
    );

    CleanScanResult {
        instance_label: targets.server_folder_name.clone(),
        selected_version: selected_instance.version.clone(),
        services,
        directories,
        registry_keys,
        start_menu_shortcuts,
        path_entries,
        excluded_note: EXCLUDED_NOTE.to_string(),
    }
}

async fn kill_instance_processes(
    app_handle: &AppHandle,
    result: &mut CleanResult,
    targets: &InstanceTargets,
) {
    // 1. 终止所有 mysqld 相关进程（通过进程名）
    logger::info(app_handle, "正在终止所有 MySQL 相关进程...");
    let _ = process_manager::execute_command("taskkill", &["/F", "/IM", "mysqld.exe", "/T"]).await;
    
    // 2. 终止特定服务的进程
    if let Some(svc) = &targets.service_name {
        logger::info(app_handle, &format!("终止实例服务进程: {}", svc));
        if let Ok(output) = process_manager::execute_command("sc", &["queryex", svc]).await {
            if output.exit_code == 0 {
                for line in output.stdout.lines() {
                    let trimmed = line.trim();
                    if let Some(pid_str) = trimmed.strip_prefix("PID") {
                        let pid = pid_str.trim().trim_start_matches(':').trim();
                        if !pid.is_empty() && pid != "0" {
                            match process_manager::execute_command("taskkill", &["/F", "/PID", pid]).await
                            {
                                Ok(out) if out.exit_code == 0 => {
                                    result.cleaned_items.push(format!(
                                        "已终止实例进程 PID {} (服务 {})",
                                        pid, svc
                                    ));
                                }
                                Ok(out) => {
                                    result.errors.push(format!(
                                        "终止 PID {} 失败: {}",
                                        pid,
                                        out.stderr.trim()
                                    ));
                                }
                                Err(e) => result.errors.push(format!("终止 PID {} 异常: {}", pid, e)),
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. 终止特定可执行文件的进程
    if let Some(bin_dir) = &targets.install_dir {
        let mysqld = Path::new(bin_dir).join("bin").join("mysqld.exe");
        let mysqld_alt = Path::new(bin_dir).join("mysqld.exe");
        let exe = if mysqld.exists() {
            Some(mysqld)
        } else if mysqld_alt.exists() {
            Some(mysqld_alt)
        } else {
            None
        };
        if let Some(path) = exe {
            let path_str = path.to_string_lossy();
            logger::info(app_handle, &format!("尝试终止实例可执行文件进程: {}", path_str));
            let _ = process_manager::execute_command(
                "wmic",
                &[
                    "process",
                    "where",
                    &format!(r#"ExecutablePath='{}'"#, path_str.replace('\\', "\\\\")),
                    "call",
                    "terminate",
                ],
            )
            .await;
        }
    }
    
    // 4. 终止 mysql.exe 客户端进程
    let _ = process_manager::execute_command("taskkill", &["/F", "/IM", "mysql.exe", "/T"]).await;
    
    // 等待进程完全终止
    logger::info(app_handle, "等待进程完全终止...");
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
}

async fn remove_residual_services(
    app_handle: &AppHandle,
    result: &mut CleanResult,
    services: Vec<String>,
) {
    if services.is_empty() {
        logger::info(app_handle, "所选实例没有关联服务，跳过服务删除");
        return;
    }

    if let Err(e) = uninstaller::stop_mysql_services(app_handle, services.clone()).await {
        result.errors.push(format!("停止服务异常: {}", e));
    }
    if let Err(e) = uninstaller::remove_mysql_services(app_handle, services.clone()).await {
        result.errors.push(format!("删除服务异常: {}", e));
    } else {
        for service in services {
            result
                .cleaned_items
                .push(format!("已删除服务: {}", service));
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
        if is_excluded_path(&dir.path) {
            logger::warn(app_handle, &format!("跳过排除路径: {}", dir.path));
            continue;
        }
        if !dir.exists {
            logger::info(app_handle, &format!("目录不存在，跳过: {}", dir.path));
            continue;
        }
        match std::fs::remove_dir_all(&dir.path) {
            Ok(_) => {
                result
                    .cleaned_items
                    .push(format!("删除目录 [{}]: {}", dir.category, dir.path));
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
    // 方法1: 尝试使用 reg delete
    let reg_result = process_manager::execute_command("reg", &["delete", key, "/f"]).await;
    if let Ok(output) = reg_result {
        if output.exit_code == 0 {
            return Ok(());
        }
    }
    
    // 方法2: 尝试使用 PowerShell 更强大的方式删除
    let key_normalized = if key.starts_with("HKLM") {
        key.replace("HKLM", "HKLM:")
    } else if key.starts_with("HKCU") {
        key.replace("HKCU", "HKCU:")
    } else {
        key.to_string()
    };
    
    let script = format!(
        r#"
        try {{
            Remove-Item -Path '{}' -Recurse -Force -ErrorAction Stop
            Write-Output 'SUCCESS'
        }} catch {{
            Write-Output "ERROR: $($_.Exception.Message)"
        }}
        "#,
        key_normalized.replace('\'', "''")
    );
    
    let ps_result = process_manager::execute_command(
        "powershell",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script]
    ).await;
    
    if let Ok(output) = ps_result {
        if output.stdout.trim() == "SUCCESS" {
            return Ok(());
        } else {
            return Err(format!(
                "PowerShell 删除失败: {}",
                if output.stderr.trim().is_empty() {
                    output.stdout.trim()
                } else {
                    output.stderr.trim()
                }
            ));
        }
    }
    
    Err("所有删除方法都失败了".to_string())
}

async fn clean_registry(
    app_handle: &AppHandle,
    result: &mut CleanResult,
    targets: &InstanceTargets,
    options: &CleanOptions,
) {
    let keys = scan_registry_keys_for_instance(targets, options).await;
    if keys.is_empty() {
        logger::info(app_handle, "未发现需要清理的注册表项");
        return;
    }

    // 优先清理 Uninstall 相关的注册表项
    let (uninstall_keys, other_keys): (Vec<_>, Vec<_>) = keys.into_iter()
        .partition(|key| key.contains("Uninstall"));
    
    // 先删除 Uninstall 键
    for key in uninstall_keys {
        match delete_registry_key(app_handle, &key).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除注册表项: {}", key));
            }
            Err(e) => {
                // 如果是路径不存在的错误，则忽略（可能父键已被删除）
                if !e.to_lowercase().contains("找不到") && !e.to_lowercase().contains("不存在") && !e.to_lowercase().contains("not found") && !e.to_lowercase().contains("does not exist") {
                    result.errors.push(format!("删除注册表 {} 失败: {}", key, e));
                    // 对于 Uninstall 键，尝试只递归删除子项
                    let _ = try_delete_registry_subkeys(app_handle, result, &key).await;
                }
            }
        }
    }
    
    // 再删除其他注册表项
    for key in other_keys {
        match delete_registry_key(app_handle, &key).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除注册表项: {}", key));
            }
            Err(e) => {
                // 如果是路径不存在的错误，则忽略（可能父键已被删除）
                if !e.to_lowercase().contains("找不到") && !e.to_lowercase().contains("不存在") && !e.to_lowercase().contains("not found") && !e.to_lowercase().contains("does not exist") {
                    result.errors.push(format!("删除注册表 {} 失败: {}", key, e));
                }
            }
        }
    }
}

/// 尝试递归删除注册表项的子键（作为备份方案）
async fn try_delete_registry_subkeys(
    app_handle: &AppHandle,
    result: &mut CleanResult,
    key: &str,
) {
    let _app_handle = app_handle; // 避免未使用变量警告
    let key_normalized = if key.starts_with("HKLM") {
        key.replace("HKLM", "HKLM:")
    } else if key.starts_with("HKCU") {
        key.replace("HKCU", "HKCU:")
    } else {
        key.to_string()
    };
    
    let script = format!(
        r#"
        try {{
            $path = '{}'
            if (Test-Path $path) {{
                Get-ChildItem -Path $path -ErrorAction SilentlyContinue | ForEach-Object {{
                    try {{
                        Remove-Item -Path $_.PSPath -Recurse -Force -ErrorAction Stop
                        Write-Output "DELETED: $($_.PSPath)"
                    }} catch {{}}
                }}
            }}
        }} catch {{}}
        "#,
        key_normalized.replace('\'', "''")
    );
    
    if let Ok(output) = process_manager::execute_command(
        "powershell",
        &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script]
    ).await {
        for line in output.stdout.lines() {
            if line.trim().starts_with("DELETED:") {
                result.cleaned_items.push(format!("删除注册表子项: {}", line.trim_start_matches("DELETED:").trim()));
            }
        }
    }
}

async fn clean_start_menu_shortcuts(
    app_handle: &AppHandle,
    result: &mut CleanResult,
    targets: &InstanceTargets,
) {
    let shortcuts = scan_start_menu_shortcuts(targets).await;
    if shortcuts.is_empty() {
        logger::info(app_handle, "未发现需要清理的开始菜单快捷方式");
        return;
    }

    for path in shortcuts {
        match std::fs::remove_dir_all(&path) {
            Ok(_) => {
                result.cleaned_items.push(format!("删除开始菜单快捷方式目录: {}", path));
                logger::info(app_handle, &format!("已删除开始菜单快捷方式: {}", path));
            }
            Err(e) => {
                result.errors.push(format!("无法删除开始菜单快捷方式目录 {}: {}", path, e));
            }
        }
    }
}

async fn clean_path_entries_for_instance(
    _app_handle: &AppHandle,
    result: &mut CleanResult,
    targets: &InstanceTargets,
) {
    let install = targets
        .install_dir
        .as_deref()
        .unwrap_or("")
        .replace('\\', "\\\\")
        .replace('\'', "''");
    let data = targets
        .program_data_dir
        .as_deref()
        .unwrap_or("")
        .replace('\\', "\\\\")
        .replace('\'', "''");

    let script = format!(
        r#"
            $install = '{}'
            $data = '{}'
            $changed = @()
            foreach ($scope in @('Machine', 'User')) {{
                $path = [Environment]::GetEnvironmentVariable('Path', $scope)
                if (-not $path) {{ continue }}
                $parts = $path -split ';' | Where-Object {{
                    $_ -and -not (
                        ($install -and $_ -like "*$install*") -or
                        ($data -and $_ -like "*$data*")
                    )
                }}
                $newPath = ($parts -join ';').TrimEnd(';')
                if ($newPath -ne $path) {{
                    [Environment]::SetEnvironmentVariable('Path', $newPath, $scope)
                    $changed += $scope
                }}
            }}
            if ($changed.Count -gt 0) {{ 'UPDATED:' + ($changed -join ',') }} else {{ 'NONE' }}
        "#,
        install, data
    );

    match process_manager::execute_command("powershell", &["-NoProfile", "-Command", &script]).await
    {
        Ok(output) if output.exit_code == 0 => {
            let stdout = output.stdout.trim();
            if stdout.starts_with("UPDATED:") {
                result.cleaned_items.push(format!(
                    "已清理实例相关 PATH: {}",
                    stdout.trim_start_matches("UPDATED:")
                ));
            }
        }
        Ok(output) => {
            result
                .errors
                .push(format!("清理 PATH 失败: {}", output.stderr.trim()));
        }
        Err(e) => result.errors.push(format!("清理 PATH 异常: {}", e)),
    }
}

pub async fn clean_mysql_residuals(
    app_handle: AppHandle,
    selected_instance: MySQLInstance,
    options: CleanOptions,
) -> CleanResult {
    let targets = build_instance_targets(&selected_instance);
    let mut result = CleanResult {
        success: true,
        message: format!("实例 {} 残留清理完成", targets.server_folder_name),
        cleaned_items: Vec::new(),
        errors: Vec::new(),
    };

    logger::info(
        &app_handle,
        &format!(
            "开始清理实例残留: {} (版本 {})",
            targets.server_folder_name, targets.version_pattern
        ),
    );
    logger::info(&app_handle, EXCLUDED_NOTE);

    // 1. 优先终止所有相关进程
    if options.kill_processes {
        kill_instance_processes(&app_handle, &mut result, &targets).await;
    }

    // 2. 停止并删除服务
    if options.remove_services {
        let services: Vec<String> = targets
            .service_name
            .clone()
            .into_iter()
            .collect();
        remove_residual_services(&app_handle, &mut result, services).await;
    }

    // 3. 再次终止可能的遗留进程，等待更长时间
    logger::info(&app_handle, "再次检查并终止遗留进程...");
    if options.kill_processes {
        let _ = process_manager::execute_command("taskkill", &["/F", "/IM", "mysqld.exe", "/T"]).await;
        let _ = process_manager::execute_command("taskkill", &["/F", "/IM", "mysql.exe", "/T"]).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
    }

    // 4. 清理注册表（可以在删除文件之前清理）
    if options.clean_registry_uninstall
        || options.clean_registry_mysql_ab
        || options.clean_registry_services
        || options.clean_odbc
        || options.clean_user_registry
    {
        clean_registry(&app_handle, &mut result, &targets, &options).await;
    }

    // 5. 最后删除文件和目录
    remove_instance_directories(&app_handle, &mut result, &targets, &options).await;

    // 6. 清理开始菜单快捷方式
    if options.clean_start_menu {
        clean_start_menu_shortcuts(&app_handle, &mut result, &targets).await;
    }

    // 7. 清理 PATH
    if options.clean_path {
        clean_path_entries_for_instance(&app_handle, &mut result, &targets).await;
    }

    if !result.errors.is_empty() {
        result.success = false;
        result.message = format!(
            "实例 {} 部分清理失败（成功 {} 项，失败 {} 项）",
            targets.server_folder_name,
            result.cleaned_items.len(),
            result.errors.len()
        );
    } else if result.cleaned_items.is_empty() {
        result.message = format!("实例 {} 未发现需要清理的残留", targets.server_folder_name);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_version_pattern_extracts_major_minor() {
        assert_eq!(derive_version_pattern("5.6.51"), "5.6");
        assert_eq!(derive_version_pattern("8.0.36"), "8.0");
    }

    #[test]
    fn derive_server_folder_from_service_name() {
        let instance = MySQLInstance {
            version: "5.6.51".to_string(),
            architecture: "x86_64".to_string(),
            status: "停止".to_string(),
            path: String::new(),
            service_name: Some("MySQL56".to_string()),
            port: Some(3306),
            is_residual: false,
        };
        assert_eq!(
            derive_server_folder_name(&instance),
            "MySQL Server 5.6"
        );
    }

    #[test]
    fn build_targets_from_bin_path() {
        let instance = MySQLInstance {
            version: "5.7.44".to_string(),
            architecture: "x86_64".to_string(),
            status: "停止".to_string(),
            path: r"C:\Program Files\MySQL\MySQL Server 5.7\bin".to_string(),
            service_name: Some("MySQL57".to_string()),
            port: None,
            is_residual: false,
        };
        let targets = build_instance_targets(&instance);
        assert_eq!(targets.server_folder_name, "MySQL Server 5.7");
        assert_eq!(
            targets.install_dir.as_deref(),
            Some(r"C:\Program Files\MySQL\MySQL Server 5.7")
        );
        assert_eq!(
            targets.program_data_dir.as_deref(),
            Some(r"C:\ProgramData\MySQL\MySQL Server 5.7")
        );
    }

    #[test]
    fn excluded_paths_block_integrated_environments() {
        assert!(is_excluded_path(r"C:\xampp\mysql"));
        assert!(!is_excluded_path(r"C:\Program Files\MySQL\MySQL Server 5.6"));
    }
}
