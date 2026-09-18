use super::super::{logger, process_manager, service_manager};
use super::super::detector_base::{self, DbInstance};
use super::super::types::{PostgresqlInstance, PostgresqlInfo};
use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

impl DbInstance for PostgresqlInstance {
    fn get_path(&self) -> &str { &self.path }
    fn set_path(&mut self, path: String) { self.path = path; }
    fn get_version(&self) -> &str { &self.version }
    fn set_version(&mut self, v: String) { self.version = v; }
    fn get_architecture(&self) -> &str { &self.architecture }
    fn set_architecture(&mut self, a: String) { self.architecture = a; }
    fn get_status(&self) -> &str { &self.status }
    fn set_status(&mut self, s: String) { self.status = s; }
    fn get_service_name(&self) -> Option<&str> { self.service_name.as_deref() }
    fn set_service_name(&mut self, n: Option<String>) { self.service_name = n; }
    fn get_port(&self) -> Option<u16> { self.port }
    fn set_port(&mut self, p: Option<u16>) { self.port = p; }
    fn get_is_residual(&self) -> bool { self.is_residual }
    fn set_is_residual(&mut self, r: bool) { self.is_residual = r; }
}

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+\.\d+)").unwrap());
static PORT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^\s*port\s*=\s*(\d+)").unwrap());

/// 从服务名（如 postgresql-x64-16）提取主版本号
fn extract_major_from_service_name(service_name: &str) -> Option<String> {
    // postgresql-x64-16 → "16"
    let parts: Vec<&str> = service_name.split('-').collect();
    if parts.len() >= 3 {
        let last = parts.last()?;
        if last.chars().all(|c| c.is_ascii_digit()) {
            return Some(last.to_string());
        }
    }
    None
}

/// 从 psql --version 输出解析版本号（如 "psql (PostgreSQL) 16.15"）
pub fn parse_version(output: &str) -> Option<String> {
    // 优先匹配 x.y.z 形式
    if let Some(captures) = Regex::new(r"(\d+\.\d+\.\d+)").unwrap().captures(output) {
        return Some(captures.get(1)?.as_str().to_string());
    }
    // 回退到 x.y 形式
    VERSION_REGEX.captures(output).map(|c| c.get(1).unwrap().as_str().to_string())
}

pub fn parse_architecture(output: &str) -> String {
    if output.contains("x86_64") || output.contains("Win64") || output.contains("x64") {
        "x86_64".to_string()
    } else if output.contains("i686") || output.contains("Win32") || output.contains("x86") {
        "x86".to_string()
    } else {
        // PostgreSQL Windows 安装器默认为 x64
        "x86_64".to_string()
    }
}

/// 查找所有 PostgreSQL 相关的 Windows 服务
pub async fn find_all_postgresql_services(_app_handle: Option<&AppHandle>) -> Vec<String> {
    service_manager::find_services_by_keywords(&["postgresql", "postgres"]).await
}

/// 从 postgresql.conf 解析端口
pub fn parse_port_from_config(content: &str) -> Option<u16> {
    for line in content.lines() {
        let trimmed = line.trim();
        // 跳过注释行
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(captures) = PORT_REGEX.captures(trimmed) {
            if let Some(port_str) = captures.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    return Some(port);
                }
            }
        }
    }
    None
}

/// 从 bin 目录推导 data 目录
/// PostgreSQL 安装结构: {install_root}/bin, {install_root}/data
fn derive_data_dir_from_bin(bin_path: &str) -> Option<String> {
    let bin = Path::new(bin_path);
    if let Some(parent) = bin.parent() {
        let data_dir = parent.join("data");
        if data_dir.is_dir() {
            return Some(data_dir.to_string_lossy().to_string());
        }
    }
    None
}

/// 查找 PostgreSQL 配置文件（postgresql.conf）
/// 优先在 data 目录中查找，回退到 bin 同级 data 目录
pub async fn get_postgresql_config_file(bin_path: &str) -> Option<PathBuf> {
    // 1. 从 bin 目录的父目录找 data 目录
    if let Some(data_dir) = derive_data_dir_from_bin(bin_path) {
        let conf = PathBuf::from(&data_dir).join("postgresql.conf");
        if conf.exists() {
            return Some(conf);
        }
    }

    // 2. 检查 EnvironmentDataDirectory 注册表项（EDB 安装器可能将 data 目录放在其他位置）
    let mut env_vars = HashMap::new();
    env_vars.insert("BIN_PATH".into(), bin_path.to_string());
    let script = r#"
        $bin = $env:BIN_PATH
        $installRoot = Split-Path $bin -Parent
        # 常见 data 目录位置
        $candidates = @(
            (Join-Path $installRoot 'data'),
            (Join-Path $env:APPDATA 'postgresql'),
            (Join-Path $env:PROGRAMDATA 'PostgreSQL')
        )
        foreach ($dir in $candidates) {
            $conf = Join-Path $dir 'postgresql.conf'
            if (Test-Path $conf) { Write-Output $conf; break }
        }
    "#;
    let output = process_manager::execute_powershell_env(script, &env_vars, 10).await;
    if let Ok(out) = output {
        let line = out.stdout.trim();
        if !line.is_empty() && Path::new(line).exists() {
            return Some(PathBuf::from(line));
        }
    }
    None
}

/// 获取 PostgreSQL 实例的端口
async fn get_postgresql_port(bin_path: &str) -> Option<u16> {
    if let Some(config_path) = get_postgresql_config_file(bin_path).await {
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            return parse_port_from_config(&content);
        }
    }
    // 回退到默认端口 5432（仅在配置文件不可用时）
    // 不自动回退，因为可能存在多实例使用不同端口
    None
}

/// 获取 PostgreSQL 实例的 data 目录
pub async fn get_postgresql_data_dir(bin_path: &str) -> Option<String> {
    // 1. 从 bin 目录推导
    if let Some(data_dir) = derive_data_dir_from_bin(bin_path) {
        return Some(data_dir);
    }
    // 2. 从配置文件中的 data_directory 设置获取
    if let Some(config_path) = get_postgresql_config_file(bin_path).await {
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("data_directory") {
                    let val = rest.trim_start_matches('=').trim().trim_matches('\'').trim_matches('"');
                    if !val.is_empty() && Path::new(val).exists() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

pub async fn start_postgresql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    service_manager::start_service(&app_handle, "PostgreSQL", &service_name)
        .await
        .map_err(Into::into)
}

pub async fn stop_postgresql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    service_manager::stop_service(&app_handle, "PostgreSQL", &service_name)
        .await
        .map_err(Into::into)
}

/// 终止指定实例目录下的 postgres.exe 进程（按可执行文件路径精准匹配）
pub async fn kill_postgresql_processes_in_dir(bin_dir: &str) -> Result<usize, String> {
    let mut env_vars = HashMap::new();
    env_vars.insert("PG_BIN_DIR".to_string(), bin_dir.trim_end_matches('\\').to_string());
    let script = r#"
        $bin = $env:PG_BIN_DIR
        if (-not $bin) { Write-Output 0; exit 0 }
        $count = 0
        Get-CimInstance Win32_Process -Filter "Name='postgres.exe' OR Name='psql.exe' OR Name='pg_ctl.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like "$bin\*" } |
            ForEach-Object {
                try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop; $count++ } catch {}
            }
        Write-Output $count
    "#;
    let output = process_manager::execute_powershell_env(script, &env_vars, 60).await
        .map_err(|e| e.to_string())?;
    let count: usize = output.stdout.trim().parse().unwrap_or(0);
    Ok(count)
}

/// 计算指定实例目录下的 postgres.exe 进程数
pub async fn count_postgresql_processes_in_dir(bin_dir: &str) -> Result<usize, String> {
    let mut env_vars = HashMap::new();
    env_vars.insert("PG_BIN_DIR".to_string(), bin_dir.trim_end_matches('\\').to_string());
    let script = r#"
        $bin = $env:PG_BIN_DIR
        if (-not $bin) { Write-Output 0; exit 0 }
        $procs = Get-CimInstance Win32_Process -Filter "Name='postgres.exe' OR Name='psql.exe' OR Name='pg_ctl.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.ExecutablePath -and $_.ExecutablePath -like "$bin\*" }
        Write-Output $procs.Count
    "#;
    let output = process_manager::execute_powershell_env(script, &env_vars, 60).await
        .map_err(|e| e.to_string())?;
    Ok(output.stdout.trim().parse::<usize>().unwrap_or(0))
}

/// 轮询等待指定实例目录下的进程退出（最多 15 秒）
pub async fn wait_postgresql_processes_gone(app_handle: &AppHandle, bin_dir: &str) {
    for _ in 0..15 {
        let remaining = count_postgresql_processes_in_dir(bin_dir).await.unwrap_or(0);
        if remaining == 0 {
            logger::info(app_handle, "实例进程已全部退出");
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    logger::warn(app_handle, "部分实例进程未能在超时时间内退出");
}

/// 主检测函数：扫描所有 PostgreSQL 实例
pub async fn detect_postgresql(app_handle: Option<&AppHandle>) -> Result<PostgresqlInfo, String> {
    let mut instances = Vec::new();
    let mut processed_paths = std::collections::HashSet::new();

    if let Some(handle) = app_handle {
        logger::info(handle, "开始检测 PostgreSQL...");
    }

    // 1. 通过 Windows 服务检测
    let pg_services = find_all_postgresql_services(app_handle).await;
    for service_name in &pg_services {
        let status = service_manager::check_service_status(service_name).await;
        if let Some(bin_path) = service_manager::get_service_binary_path(service_name).await {
            if !processed_paths.contains(&bin_path) {
                processed_paths.insert(bin_path.clone());
                let bin_dir = Path::new(&bin_path);
                let psql_exe = bin_dir.join("psql.exe");
                let postgres_exe = bin_dir.join("postgres.exe");

                let exe_to_check = if psql_exe.exists() { psql_exe }
                    else if postgres_exe.exists() { postgres_exe }
                    else {
                        if let Some(handle) = app_handle {
                            logger::warn(handle, &format!("未找到可执行文件，跳过此路径: {}", bin_path));
                        }
                        continue;
                    };

                // 从服务名提取主版本号作为兜底
                let svc_version = extract_major_from_service_name(service_name);
                let result = process_manager::execute_command(exe_to_check.to_str().unwrap_or(""), &["--version"]).await;
                let (version, arch) = if let Ok(output) = result {
                    if output.exit_code == 0 {
                        let v = parse_version(&output.stdout).unwrap_or_else(|| svc_version.clone().unwrap_or_default());
                        let a = parse_architecture(&output.stdout);
                        (v, a)
                    } else {
                        (svc_version.clone().unwrap_or_else(|| detector_base::VERSION_UNKNOWN.to_string()), detector_base::ARCH_UNKNOWN.to_string())
                    }
                } else {
                    (svc_version.clone().unwrap_or_else(|| detector_base::VERSION_UNKNOWN.to_string()), detector_base::ARCH_UNKNOWN.to_string())
                };

                let port = get_postgresql_port(&bin_path).await;
                let data_dir = get_postgresql_data_dir(&bin_path).await;

                instances.push(PostgresqlInstance {
                    version,
                    architecture: arch,
                    status,
                    path: bin_path,
                    service_name: Some(service_name.clone()),
                    port,
                    data_dir,
                    is_residual: false,
                });
            }
        } else {
            let already_added = instances.iter().any(|inst| inst.service_name.as_deref() == Some(service_name));
            if !already_added {
                instances.push(PostgresqlInstance {
                    version: extract_major_from_service_name(service_name).unwrap_or_else(|| detector_base::VERSION_UNKNOWN.to_string()),
                    architecture: detector_base::ARCH_UNKNOWN.to_string(),
                    status,
                    path: String::new(),
                    service_name: Some(service_name.clone()),
                    port: None,
                    data_dir: None,
                    is_residual: false,
                });
            }
        }
    }

    // 2. 扫描常见安装目录
    let dir_paths = detector_base::scan_directories(
        &[r"C:\Program Files\PostgreSQL", r"C:\Program Files (x86)\PostgreSQL"],
        &["psql.exe", "postgres.exe"],
    ).await;
    for bin_path in dir_paths {
        if processed_paths.contains(&bin_path) { continue; }
        processed_paths.insert(bin_path.clone());
        let bin_dir = Path::new(&bin_path);
        let psql_exe = bin_dir.join("psql.exe");
        let postgres_exe = bin_dir.join("postgres.exe");
        let exe_to_check = if psql_exe.exists() { psql_exe } else { postgres_exe };

        let result = process_manager::execute_command(exe_to_check.to_str().unwrap_or(""), &["--version"]).await;
        if let Ok(output) = result {
            if output.exit_code == 0 {
                let version = parse_version(&output.stdout).unwrap_or(detector_base::ARCH_UNKNOWN.to_string());
                let arch = parse_architecture(&output.stdout);
                let port = get_postgresql_port(&bin_path).await;
                let data_dir = get_postgresql_data_dir(&bin_path).await;
                instances.push(PostgresqlInstance {
                    version,
                    architecture: arch,
                    status: detector_base::STATUS_NO_SERVICE.to_string(),
                    path: bin_path,
                    service_name: None,
                    port,
                    data_dir,
                    is_residual: false,
                });
            }
        }
    }

    // 3. 去重
    instances = detector_base::dedupe_instances(instances);

    // 4. 验证每个实例是否有效（标记残留）
    instances = detector_base::validate_instances(instances).await;

    let total_count = instances.len() as i32;
    if let Some(handle) = app_handle {
        logger::info(handle, &format!("检测完成，发现 {} 个 PostgreSQL 实例", total_count));
    }

    Ok(PostgresqlInfo { instances, total_count })
}
