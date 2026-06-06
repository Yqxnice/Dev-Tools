use super::super::{logger, process_manager};
use super::super::types::{MySQLInstance, MySQLInfo};
use regex::Regex;
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+\.\d+\.\d+)").unwrap());
static BINARY_PATH_REGEX_QUOTED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"BINARY_PATH_NAME\s*:\s*"([^"]+)"#).unwrap());
static BINARY_PATH_REGEX_UNQUOTED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"BINARY_PATH_NAME\s*:\s*(\S+)"#).unwrap());
static PORT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^port\s*=\s*(\d+)").unwrap());
static DEFAULTS_FILE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)--defaults-file=(?:"([^"]+)"|([^\s"]+))"#).unwrap()
});

fn collect_config_file_candidates(bin_path: &str, service_name: Option<&str>) -> Vec<PathBuf> {
    let bin_dir = Path::new(bin_path);
    let mut config_files = Vec::new();

    if let Some(parent) = bin_dir.parent() {
        config_files.push(parent.join("my.ini"));
        config_files.push(parent.join("my.cnf"));
    }
    config_files.push(bin_dir.join("my.ini"));
    config_files.push(bin_dir.join("my.cnf"));

    if let Some(svc) = service_name {
        let version_str = if svc.starts_with("MySQL") && svc.len() > 5 {
            let num_part = &svc[5..];
            if num_part.len() >= 2 {
                let major = &num_part[0..1];
                let minor = &num_part[1..2];
                format!("MySQL Server {}.{}", major, minor)
            } else {
                svc.to_string()
            }
        } else {
            svc.to_string()
        };

        config_files.push(PathBuf::from(r"C:\ProgramData\MySQL").join(&version_str).join("my.ini"));
        config_files.push(PathBuf::from(r"C:\ProgramData\MySQL").join(&version_str).join("my.cnf"));
    }

    config_files.push(PathBuf::from(r"C:\ProgramData\MySQL").join("my.ini"));
    config_files.push(PathBuf::from(r"C:\ProgramData\MySQL").join("my.cnf"));

    let program_data_mysql = PathBuf::from(r"C:\ProgramData\MySQL");
    if program_data_mysql.exists() {
        if let Ok(entries) = std::fs::read_dir(&program_data_mysql) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    config_files.push(path.join("my.ini"));
                    config_files.push(path.join("my.cnf"));
                }
            }
        }
    }

    config_files
}

pub async fn get_mysql_config_file(service_name: Option<&str>, bin_path: &str) -> Option<PathBuf> {
    if let Some(svc) = service_name {
        if let Ok(output) = process_manager::execute_command("sc", &["qc", svc]).await {
            if output.exit_code == 0 {
                if let Some(captures) = DEFAULTS_FILE_REGEX.captures(&output.stdout) {
                    let path_str = captures
                        .get(1)
                        .or_else(|| captures.get(2))
                        .map(|m| m.as_str())?;
                    let config_path = PathBuf::from(path_str);
                    if config_path.exists() {
                        return Some(config_path);
                    }
                }
            }
        }
    }

    for config_path in collect_config_file_candidates(bin_path, service_name) {
        if config_path.exists() {
            return Some(config_path);
        }
    }

    None
}

pub fn parse_port_from_config(content: &str) -> Option<u16> {
    let mut in_mysqld = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed[1..trimmed.len() - 1].trim().to_lowercase();
            in_mysqld = section == "mysqld";
            continue;
        }
        if in_mysqld {
            if let Some(captures) = PORT_REGEX.captures(trimmed) {
                if let Some(port_str) = captures.get(1) {
                    if let Ok(port) = port_str.as_str().parse::<u16>() {
                        return Some(port);
                    }
                }
            }
        }
    }
    None
}

async fn collect_mysqld_pids() -> Vec<u32> {
    let mut pids = Vec::new();
    if let Ok(output) = process_manager::execute_command(
        "tasklist",
        &["/FI", "IMAGENAME eq mysqld.exe", "/FO", "CSV", "/NH"],
    )
    .await
    {
        if output.exit_code == 0 {
            for line in output.stdout.lines() {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    let pid_str = parts[1].trim().trim_matches('"');
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }
    }
    pids
}

fn parse_listening_port_for_pid(line: &str, pid: u32) -> Option<u16> {
    if !line.contains("LISTENING") {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let line_pid = parts.last()?.parse::<u32>().ok()?;
    if line_pid != pid {
        return None;
    }
    let local_addr = parts.get(1)?;
    let port_str = local_addr.rsplit(':').next()?;
    port_str.parse().ok()
}

async fn get_port_by_mysqld_pid() -> Option<u16> {
    let pids = collect_mysqld_pids().await;
    if pids.is_empty() {
        return None;
    }

    if let Ok(output) = process_manager::execute_command("netstat", &["-ano"]).await {
        if output.exit_code == 0 {
            for pid in &pids {
                for line in output.stdout.lines() {
                    if let Some(port) = parse_listening_port_for_pid(line, *pid) {
                        return Some(port);
                    }
                }
            }
        }
    }
    None
}

// 获取 MySQL 实例的端口
async fn get_mysql_port(
    bin_path: &str,
    service_name: Option<&str>,
) -> Option<u16> {
    for config_path in collect_config_file_candidates(bin_path, service_name) {
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Some(port) = parse_port_from_config(&content) {
                    return Some(port);
                }
            }
        }
    }

    get_port_by_mysqld_pid().await
}

fn merge_instance(existing: &mut MySQLInstance, incoming: MySQLInstance) {
    if existing.path.is_empty() && !incoming.path.is_empty() {
        existing.path = incoming.path;
    }
    if existing.version.is_empty()
        || existing.version == "未知版本"
        || existing.version == "未知"
    {
        if !incoming.version.is_empty() && incoming.version != "未知版本" {
            existing.version = incoming.version;
        }
    }
    if existing.architecture.is_empty() || existing.architecture == "未知" {
        if !incoming.architecture.is_empty() && incoming.architecture != "未知" {
            existing.architecture = incoming.architecture;
        }
    }
    if existing.port.is_none() {
        existing.port = incoming.port;
    }
    if existing.service_name.is_none() {
        existing.service_name = incoming.service_name;
    }
    if existing.status == "未安装" || existing.status == "未安装服务" {
        if incoming.status != "未安装" {
            existing.status = incoming.status;
        }
    }
}

fn dedupe_instances(mut instances: Vec<MySQLInstance>) -> Vec<MySQLInstance> {
    let mut result: Vec<MySQLInstance> = Vec::new();

    for inst in instances.drain(..) {
        if inst.status == "未安装" && inst.path.is_empty() && inst.service_name.is_none() {
            continue;
        }

        if let Some(existing) = result.iter_mut().find(|e| {
            (!inst.path.is_empty() && e.path == inst.path)
                || (inst.service_name.is_some() && e.service_name == inst.service_name)
        }) {
            merge_instance(existing, inst);
        } else {
            result.push(inst);
        }
    }

    result
}

pub fn parse_version(output: &str) -> Option<String> {
    if let Some(captures) = VERSION_REGEX.captures(output) {
        captures.get(1).map(|m| m.as_str().to_string())
    } else {
        None
    }
}

pub fn parse_architecture(output: &str) -> String {
    if output.contains("x86_64") || output.contains("Win64") {
        "x86_64".to_string()
    } else if output.contains("i686") || output.contains("Win32") {
        "x86".to_string()
    } else {
        "unknown".to_string()
    }
}

pub async fn check_service_exists(service_name: &str) -> bool {
    let result = process_manager::execute_command("sc", &["query", service_name]).await;
    if let Ok(output) = result {
        // If the service exists, sc query will return something that's not "The specified service does not exist"
        !output.stderr.contains("The specified service does not exist") && !output.stdout.contains("The specified service does not exist")
    } else {
        false
    }
}

pub async fn check_service_status(service_name: &str) -> String {
    let result = process_manager::execute_command("sc", &["query", service_name]).await;
    if let Ok(output) = result {
        if output.stdout.contains("RUNNING") {
            return "启动".to_string();
        } else if output.stdout.contains("STOPPED") {
            return "停止".to_string();
        }
    }
    "未安装".to_string()
}

pub async fn is_instance_valid(instance: &MySQLInstance) -> bool {
    // Check if path exists
    let path_valid = !instance.path.is_empty() && Path::new(&instance.path).exists();
    
    // Check if service exists
    let service_valid = if let Some(svc) = &instance.service_name {
        check_service_exists(svc).await
    } else {
        false
    };
    
    path_valid || service_valid
}

pub async fn find_all_mysql_services(_app_handle: Option<&AppHandle>) -> Vec<String> {
    let mut services = Vec::new();
    let result = process_manager::execute_command("sc", &["query", "type=", "service", "state=", "all"]).await;
    
    if let Ok(output) = result {
        let lines: Vec<&str> = output.stdout.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.starts_with("SERVICE_NAME:") {
                let service_name = line.strip_prefix("SERVICE_NAME:").unwrap_or("").trim();
                let service_name_lower = service_name.to_lowercase();
                if service_name_lower.contains("mysql") || 
                   service_name_lower.contains("mariadb") {
                    services.push(service_name.to_string());
                }
            }
            i += 1;
        }
    }
    services
}

async fn get_service_binary_path(service_name: &str) -> Option<String> {
    let result = process_manager::execute_command("sc", &["qc", service_name]).await;
    if let Ok(output) = result {
        if let Some(captures) = BINARY_PATH_REGEX_QUOTED.captures(&output.stdout) {
            let path_str = captures.get(1)?.as_str();
            if let Some(parent) = Path::new(path_str).parent() {
                return Some(parent.to_str().unwrap_or("").to_string());
            }
        }
        if let Some(captures) = BINARY_PATH_REGEX_UNQUOTED.captures(&output.stdout) {
            let path_str = captures.get(1)?.as_str();
            if let Some(parent) = Path::new(path_str).parent() {
                return Some(parent.to_str().unwrap_or("").to_string());
            }
        }
    }
    None
}

async fn scan_directories_for_mysql() -> Vec<String> {
    let mut paths = Vec::new();
    
    let common_paths = vec![
        r"C:\Program Files\MySQL",
        r"C:\Program Files (x86)\MySQL",
        r"C:\Program Files\MariaDB",
        r"C:\Program Files (x86)\MariaDB",
        r"C:\xampp\mysql",
        r"C:\wamp64\bin\mysql",
        r"C:\wamp\bin\mysql",
        r"C:\appserv\mysql",
    ];

    for base_path in common_paths {
        let base = Path::new(base_path);
        if base.exists() && base.is_dir() {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let bin_path = path.join("bin");
                        if bin_path.exists() {
                            let mysql_exe = bin_path.join("mysql.exe");
                            let mysqld_exe = bin_path.join("mysqld.exe");
                            if mysql_exe.exists() || mysqld_exe.exists() {
                                paths.push(bin_path.to_str().unwrap_or("").to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    paths
}

pub async fn start_mysql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    logger::info(&app_handle, &format!("正在启动 MySQL 服务: {}", service_name));
    let result = process_manager::execute_command("net", &["start", &service_name]).await;
    match result {
        Ok(output) => {
            if output.exit_code == 0 {
                logger::info(&app_handle, &format!("MySQL 服务 {} 启动成功", service_name));
                Ok(())
            } else {
                let error_msg = if !output.stderr.is_empty() {
                    output.stderr
                } else if !output.stdout.is_empty() {
                    output.stdout
                } else {
                    "启动失败，未知错误".to_string()
                };
                logger::error(&app_handle, &format!("MySQL 服务 {} 启动失败: {}", service_name, error_msg));
                Err(error_msg)
            }
        }
        Err(e) => {
            logger::error(&app_handle, &format!("启动 MySQL 服务异常: {}", e));
            Err(e)
        }
    }
}

pub async fn stop_mysql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    logger::info(&app_handle, &format!("正在停止 MySQL 服务: {}", service_name));
    let result = process_manager::execute_command("net", &["stop", &service_name]).await;
    match result {
        Ok(output) => {
            if output.exit_code == 0 {
                logger::info(&app_handle, &format!("MySQL 服务 {} 停止成功", service_name));
                Ok(())
            } else {
                let error_msg = if !output.stderr.is_empty() {
                    output.stderr
                } else if !output.stdout.is_empty() {
                    output.stdout
                } else {
                    "停止失败，未知错误".to_string()
                };
                logger::error(&app_handle, &format!("MySQL 服务 {} 停止失败: {}", service_name, error_msg));
                Err(error_msg)
            }
        }
        Err(e) => {
            logger::error(&app_handle, &format!("停止 MySQL 服务异常: {}", e));
            Err(e)
        }
    }
}

pub async fn detect_all_mysql(app_handle: Option<&AppHandle>) -> MySQLInfo {
    let mut instances = Vec::new();
    let mut processed_paths = std::collections::HashSet::new();
    
    if let Some(handle) = app_handle {
        logger::info(handle, "开始检测 MySQL...");
    }
    let mysql_services = find_all_mysql_services(app_handle).await;
    
    for service_name in &mysql_services {
        let status = check_service_status(service_name).await;
        
        if let Some(bin_path) = get_service_binary_path(service_name).await {
            if !processed_paths.contains(&bin_path) {
                processed_paths.insert(bin_path.clone());
                
                let bin_dir = Path::new(&bin_path);
                let mysql_exe = bin_dir.join("mysql.exe");
                let mysqld_exe = bin_dir.join("mysqld.exe");
                
                let exe_to_check = if mysql_exe.exists() { 
                    mysql_exe 
                } else if mysqld_exe.exists() { 
                    mysqld_exe 
                } else { 
                    if let Some(handle) = app_handle {
                        logger::warn(handle, &format!("未找到可执行文件，跳过此路径: {}", bin_path));
                    }
                    continue; 
                };
                
                let result = process_manager::execute_command(
                    exe_to_check.to_str().unwrap(),
                    &["--version"]
                ).await;
                
                if let Ok(output) = result {
                    if output.exit_code == 0 {
                        let version = parse_version(&output.stdout).unwrap_or("未知".to_string());
                        let arch = parse_architecture(&output.stdout);
                        let port = get_mysql_port(&bin_path, Some(service_name)).await;
                        
                        instances.push(MySQLInstance {
                            version,
                            architecture: arch,
                            status,
                            path: bin_path,
                            service_name: Some(service_name.clone()),
                            port,
                            is_residual: false,
                        });
                    }
                }
            }
        } else {
            let already_added = instances.iter()
                .any(|inst| inst.service_name.as_deref() == Some(service_name));
            if !already_added {
                instances.push(MySQLInstance {
                    version: "未知版本".to_string(),
                    architecture: "未知".to_string(),
                    status,
                    path: String::new(),
                    service_name: Some(service_name.clone()),
                    port: None,
                    is_residual: false,
                });
            }
        }
    }

    let dir_paths = scan_directories_for_mysql().await;
    
    for bin_path in dir_paths {
        if processed_paths.contains(&bin_path) {
            continue;
        }
        processed_paths.insert(bin_path.clone());
        
        let bin_dir = Path::new(&bin_path);
        let mysql_exe = bin_dir.join("mysql.exe");
        let mysqld_exe = bin_dir.join("mysqld.exe");
        
        let exe_to_check = if mysql_exe.exists() { 
            mysql_exe 
        } else { 
            mysqld_exe 
        };
        
        let result = process_manager::execute_command(
            exe_to_check.to_str().unwrap(),
            &["--version"]
        ).await;
        
        if let Ok(output) = result {
            if output.exit_code == 0 {
                let version = parse_version(&output.stdout).unwrap_or("未知".to_string());
                let arch = parse_architecture(&output.stdout);
                let port = get_mysql_port(&bin_path, None).await;
                
                instances.push(MySQLInstance {
                    version,
                    architecture: arch,
                    status: "未安装服务".to_string(),
                    path: bin_path,
                    service_name: None,
                    port,
                    is_residual: false,
                });
            }
        }
    }

    if instances.is_empty() {
        let search_paths = vec![
            r"C:\Program Files\MySQL",
            r"C:\ProgramData\MySQL",
        ];
        let common_service_names = vec![
            "MySQL80",
            "MySQL57",
            "MySQL56",
            "MySQL55",
            "MySQL",
        ];

        for base_path in search_paths {
            let base = Path::new(base_path);
            if base.exists() {
                if let Ok(entries) = std::fs::read_dir(base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let bin_path = path.join("bin");
                            if bin_path.exists() && !processed_paths.contains(bin_path.to_str().unwrap_or("")) {
                                let mysql_exe = bin_path.join("mysql.exe");
                                let mysqld_exe = bin_path.join("mysqld.exe");

                                for exe in [mysql_exe, mysqld_exe] {
                                    if exe.exists() {
                                        let result = process_manager::execute_command(
                                            exe.to_str().unwrap(),
                                            &["--version"]
                                        ).await;
                                        if let Ok(output) = result {
                                            if output.exit_code == 0 {
                                                if let Some(version) = parse_version(&output.stdout) {
                                                    let arch = parse_architecture(&output.stdout);
                                                    let mut found_service = None;
                                                    
                                                    for service in &common_service_names {
                                                        let status = check_service_status(service).await;
                                                        if status != "未安装" {
                                                            found_service = Some(service.to_string());
                                                            let port = get_mysql_port(bin_path.to_str().unwrap(), Some(service)).await;
                                                            instances.push(MySQLInstance {
                                                                version: version.clone(),
                                                                architecture: arch.clone(),
                                                                status,
                                                                path: bin_path.to_str().unwrap().to_string(),
                                                                service_name: Some(service.to_string()),
                                                                port,
                                                                is_residual: false,
                                                            });
                                                            break;
                                                        }
                                                    }
                                                    
                                                    if found_service.is_none() {
                                                        let port = get_mysql_port(bin_path.to_str().unwrap(), None).await;
                                                        instances.push(MySQLInstance {
                                                            version,
                                                            architecture: arch,
                                                            status: "未安装服务".to_string(),
                                                            path: bin_path.to_str().unwrap().to_string(),
                                                            service_name: None,
                                                            port,
                                                            is_residual: false,
                                                        });
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for service in &common_service_names {
            let status = check_service_status(service).await;
            if status != "未安装" {
                let exists = instances.iter().any(|inst| inst.service_name.as_deref() == Some(service));
                if !exists {
                    instances.push(MySQLInstance {
                        version: "未知版本".to_string(),
                        architecture: "未知".to_string(),
                        status,
                        path: String::new(),
                        service_name: Some(service.to_string()),
                        port: None,
                        is_residual: false,
                    });
                }
            }
        }
    }

    instances = dedupe_instances(instances);

    // Validate each instance to check if it's residual
    let mut validated_instances = Vec::new();
    for mut instance in instances {
        let is_valid = is_instance_valid(&instance).await;
        instance.is_residual = !is_valid;
        validated_instances.push(instance);
    }
    instances = validated_instances;

    let total_count = instances.len() as i32;

    if let Some(handle) = app_handle {
        logger::info(handle, &format!("检测完成，发现 {} 个实例", total_count));
    }

    MySQLInfo {
        instances,
        total_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_port_from_mysqld_section() {
        let content = r#"
[mysql]
port=3307
[mysqld]
port=3308
"#;
        assert_eq!(parse_port_from_config(content), Some(3308));
    }

    #[test]
    fn parse_port_no_mysqld_section() {
        let content = r#"
[mysql]
port=3307
[client]
port=3309
"#;
        assert_eq!(parse_port_from_config(content), None);
    }

    #[test]
    fn parse_port_invalid_number() {
        let content = r#"
[mysqld]
port=not-a-number
"#;
        assert_eq!(parse_port_from_config(content), None);
    }

    #[test]
    fn parse_version_extracts_semver() {
        assert_eq!(
            parse_version("mysql  Ver 8.0.36 for Win64 on x86_64"),
            Some("8.0.36".to_string())
        );
        assert_eq!(
            parse_version("mysqld  Ver 5.7.44 for Win64 on x86_64"),
            Some("5.7.44".to_string())
        );
        assert_eq!(
            parse_version("mariadb  Ver 10.6.12-MariaDB for Win64 on x86_64"),
            Some("10.6.12".to_string())
        );
    }

    #[test]
    fn parse_version_invalid_format() {
        assert_eq!(parse_version("invalid version string"), None);
        assert_eq!(parse_version("mysql Ver for Win64"), None);
    }

    #[test]
    fn parse_architecture_detects_x86_64() {
        assert_eq!(parse_architecture("mysql  Ver 8.0.36 for Win64 on x86_64"), "x86_64");
        assert_eq!(parse_architecture("mysql  Ver 8.0.36 for Win64"), "x86_64");
    }

    #[test]
    fn parse_architecture_detects_x86() {
        assert_eq!(parse_architecture("mysql  Ver 5.6.51 for Win32 on i686"), "x86");
        assert_eq!(parse_architecture("mysql  Ver 5.6.51 for Win32"), "x86");
    }

    #[test]
    fn parse_architecture_unknown() {
        assert_eq!(parse_architecture("unknown architecture"), "unknown");
    }

    #[test]
    fn dedupe_merges_same_path() {
        let instances = vec![
            MySQLInstance {
                version: "8.0.36".to_string(),
                architecture: "x86_64".to_string(),
                status: "启动".to_string(),
                path: r"C:\mysql\bin".to_string(),
                service_name: Some("MySQL80".to_string()),
                port: None,
                is_residual: false,
            },
            MySQLInstance {
                version: "8.0.36".to_string(),
                architecture: "x86_64".to_string(),
                status: "启动".to_string(),
                path: r"C:\mysql\bin".to_string(),
                service_name: Some("MySQL80".to_string()),
                port: Some(3306),
                is_residual: false,
            },
        ];
        let deduped = dedupe_instances(instances);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].port, Some(3306));
    }

    #[test]
    fn dedupe_preserves_different_instances() {
        let instances = vec![
            MySQLInstance {
                version: "8.0.36".to_string(),
                architecture: "x86_64".to_string(),
                status: "启动".to_string(),
                path: r"C:\mysql8\bin".to_string(),
                service_name: Some("MySQL80".to_string()),
                port: Some(3306),
                is_residual: false,
            },
            MySQLInstance {
                version: "5.7.44".to_string(),
                architecture: "x86_64".to_string(),
                status: "停止".to_string(),
                path: r"C:\mysql57\bin".to_string(),
                service_name: Some("MySQL57".to_string()),
                port: Some(3307),
                is_residual: false,
            },
        ];
        let deduped = dedupe_instances(instances);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn merge_instance_updates_missing_fields() {
        let mut existing = MySQLInstance {
            version: "未知版本".to_string(),
            architecture: "未知".to_string(),
            status: "未安装".to_string(),
            path: String::new(),
            service_name: None,
            port: None,
            is_residual: false,
        };
        let incoming = MySQLInstance {
            version: "8.0.36".to_string(),
            architecture: "x86_64".to_string(),
            status: "启动".to_string(),
            path: r"C:\mysql\bin".to_string(),
            service_name: Some("MySQL80".to_string()),
            port: Some(3306),
            is_residual: false,
        };
        merge_instance(&mut existing, incoming);
        assert_eq!(existing.version, "8.0.36");
        assert_eq!(existing.architecture, "x86_64");
        assert_eq!(existing.status, "启动");
        assert_eq!(existing.path, r"C:\mysql\bin");
        assert_eq!(existing.service_name, Some("MySQL80".to_string()));
        assert_eq!(existing.port, Some(3306));
    }
}
