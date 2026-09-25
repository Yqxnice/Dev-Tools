use super::super::{detector_base, logger, process_manager, types};
use super::detector;
use std::path::PathBuf;
use tauri::AppHandle;

/// Drop guard 确保临时配置文件被清理（即使 panic 也执行）
struct TempConfigGuard {
    path: Option<PathBuf>,
}

impl TempConfigGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn path(&self) -> &PathBuf {
        self.path.as_ref().unwrap()
    }
}

impl Drop for TempConfigGuard {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// 创建临时 MySQL 配置文件，避免密码出现在进程参数中
async fn create_temp_mysql_config(password: &str) -> Result<TempConfigGuard, String> {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join(format!("devtools_mysql_{}.cnf", std::process::id()));
    let config_content = format!("[client]\nuser=root\npassword={}\n", password);
    tokio::fs::write(&config_path, &config_content).await
        .map_err(|e| format!("创建临时配置文件失败: {}", e))?;
    Ok(TempConfigGuard::new(config_path))
}

// 在 Windows 上隐藏控制台窗口，避免闪烁
#[cfg(target_os = "windows")]
fn hide_console_window(command: &mut tokio::process::Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
}

async fn stop_mysql_service(app_handle: &AppHandle, service_name: &str) -> Result<(), String> {
    logger::info(app_handle, &format!("正在停止 MySQL 服务: {}", service_name));
    match process_manager::execute_command("net", &["stop", service_name]).await {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::warn(app_handle, &format!("停止服务警告: {}", output.stderr));
            }
        }
        Err(e) => {
            logger::warn(app_handle, &format!("停止服务失败: {}", e));
        }
    }
    // 轮询等待服务真正进入停止状态（最多 30 秒），替代固定 sleep
    wait_for_service_state(app_handle, service_name, false, 30).await;
    Ok(())
}

/// 轮询等待服务达到目标状态（running=true 等待"启动"，running=false 等待"停止"/"未安装"）
async fn wait_for_service_state(
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
async fn wait_for_port_ready(app_handle: &AppHandle, port: u16, timeout_secs: u64) -> bool {
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

/// 终止指定实例目录下的 mysqld/mysql 进程（按可执行文件路径精准匹配，
/// 不影响同机其他 MySQL/MariaDB 实例），并轮询等待进程退出。
async fn kill_mysqld_processes(app_handle: &AppHandle, bin_dir: &str) -> Result<(), String> {
    if bin_dir.is_empty() {
        logger::warn(app_handle, "实例路径未知，跳过进程终止（避免误杀其他实例）");
        return Ok(());
    }
    logger::info(app_handle, &format!("正在终止该实例的 MySQL 进程（目录: {}）...", bin_dir));
    match process_manager::kill_mysql_processes_in_dir(bin_dir).await {
        Ok(n) => logger::info(app_handle, &format!("已终止 {} 个实例进程", n)),
        Err(e) => logger::warn(app_handle, &format!("终止进程失败: {}", e)),
    }
    // 轮询等待进程完全退出（最多 15 秒）
    for _ in 0..15 {
        let remaining = match process_manager::count_mysql_processes_in_dir(bin_dir).await {
            Ok(n) => n,
            Err(e) => {
                // 查询失败时不提前退出（避免误判进程已退出），继续等待
                logger::warn(app_handle, &format!("查询进程数失败，继续等待: {}", e));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };
        if remaining == 0 {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    logger::warn(app_handle, "部分实例进程未能在超时时间内退出");
    Ok(())
}

/// 重置流程失败后的兜底恢复：杀掉 --skip-grant-tables 临时实例，
/// 并把原 Windows 服务以正常认证模式重新启动，避免服务长时间停摆或停留在无认证状态。
/// 恢复动作的成败不掩盖原始错误；恢复失败时给出手动处置指引。
async fn restore_service_after_failure(
    app_handle: &AppHandle,
    service_name: &str,
    bin_dir: &str,
    mut child: Option<&mut tokio::process::Child>,
) {
    logger::warn(app_handle, "重置流程异常中断，开始自动恢复：停止无授权模式实例并重启原服务...");
    if let Some(ref mut c) = child {
        let _ = c.kill().await;
    }
    let _ = kill_mysqld_processes(app_handle, bin_dir).await;
    let _ = start_mysql_service(app_handle, service_name).await;
    let restarted = wait_for_service_state(app_handle, service_name, true, 30).await;
    if !restarted {
        logger::error(app_handle, &format!(
            "自动恢复服务失败，请手动在管理员终端执行 `net start {}` 启动服务", service_name));
    } else {
        logger::info(app_handle, "自动恢复完成：原服务已重新启动（正常认证模式）");
    }
}

/// 安全地转义 MySQL 密码字符串
/// 遵循 MySQL 字符串转义规则：https://dev.mysql.com/doc/refman/8.0/en/string-literals.html
pub fn escape_mysql_password(password: &str) -> String {
    let mut result = String::with_capacity(password.len() * 2);
    for c in password.chars() {
        match c {
            '\'' => result.push_str("''"),
            '\"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\0' => result.push_str("\\0"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x1a' => result.push_str("\\Z"),
            _ => result.push(c),
        }
    }
    result
}

/// 使用 process_manager 中的密码强度验证
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    process_manager::validate_password_strength(password)
        .map_err(|e| e.to_user_message())
}

/// 判断 MySQL 版本是否为 8.0 及以上（包括 9.0+ Innovation 系列）。
/// MySQL 8.0 起移除了 PASSWORD() 函数，9.0+ 同样不支持，必须用 ALTER USER。
fn is_mysql_8_or_higher(version: &str) -> bool {
    version
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .map_or(false, |m| m >= 8)
}

/// 判断 MySQL 版本是否为 9.0 及以上。
/// MySQL 9.0 彻底移除了 mysql_native_password 插件，只能使用 caching_sha2_password。
fn is_mysql_9_or_higher(version: &str) -> bool {
    version
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .map_or(false, |m| m >= 9)
}

fn resolve_port(instance: &types::MySQLInstance, override_port: Option<u16>) -> Option<u16> {
    override_port.or(instance.port)
}

pub fn build_password_reset_sql(version: &str, new_password: &str) -> String {
    let escaped_password = escape_mysql_password(new_password);

    if is_mysql_9_or_higher(version) {
        // MySQL 9.0+: mysql_native_password 插件已彻底移除，必须使用 caching_sha2_password
        [
            "UPDATE mysql.user SET authentication_string='', plugin='caching_sha2_password' WHERE User='root';"
                .to_string(),
            "SELECT ROW_COUNT() AS affected_rows;".to_string(),
            "FLUSH PRIVILEGES;".to_string(),
            format!("ALTER USER 'root'@'localhost' IDENTIFIED BY '{}';", escaped_password),
            format!("ALTER USER 'root'@'127.0.0.1' IDENTIFIED BY '{}';", escaped_password),
            format!("ALTER USER 'root'@'%' IDENTIFIED BY '{}';", escaped_password),
        ]
        .join(" ")
    } else if is_mysql_8_or_higher(version) {
        // MySQL 8.0-8.x: mysql_native_password 仍可用（已弃用），保持兼容
        [
            "UPDATE mysql.user SET authentication_string='', plugin='mysql_native_password' WHERE User='root';"
                .to_string(),
            "SELECT ROW_COUNT() AS affected_rows;".to_string(),
            "FLUSH PRIVILEGES;".to_string(),
            format!(
                "ALTER USER 'root'@'localhost' IDENTIFIED WITH mysql_native_password BY '{}';",
                escaped_password
            ),
            format!(
                "ALTER USER 'root'@'127.0.0.1' IDENTIFIED WITH mysql_native_password BY '{}';",
                escaped_password
            ),
            format!(
                "ALTER USER 'root'@'%' IDENTIFIED WITH mysql_native_password BY '{}';",
                escaped_password
            ),
        ]
        .join(" ")
    } else if version.starts_with("5.7") {
        [
            format!(
                "UPDATE mysql.user SET authentication_string=PASSWORD('{}'), plugin='mysql_native_password', password_expired='N' WHERE User='root';",
                escaped_password
            ),
            "SELECT ROW_COUNT() AS affected_rows;".to_string(),
            "FLUSH PRIVILEGES;".to_string(),
        ]
        .join(" ")
    } else {
        [
            format!(
                "UPDATE mysql.user SET Password=PASSWORD('{}'), plugin='mysql_native_password', password_expired='N' WHERE User='root';",
                escaped_password
            ),
            "SELECT ROW_COUNT() AS affected_rows;".to_string(),
            "FLUSH PRIVILEGES;".to_string(),
        ]
        .join(" ")
    }
}

/// 构建容错版修改密码SQL - 逐个尝试修改，失败不影响后续
pub fn build_safe_change_password_sql(version: &str, new_password: &str) -> String {
    let escaped = escape_mysql_password(new_password);

    if is_mysql_8_or_higher(version) {
        // MySQL 8.0: 不再使用已弃用的 PASSWORD() 函数
        // 先查询用户，再动态生成 ALTER USER 语句
        format!(
            "-- MySQL 8.0+ 安全密码修改方式\n\
             SET @new_password = '{}';\n\
             -- 生成 ALTER USER 语句修改所有 root 用户\n\
             SET @sql = NULL;\n\
             SELECT GROUP_CONCAT(CONCAT('ALTER USER ''', User, '''@''', Host, ''' IDENTIFIED BY ', QUOTE(@new_password), ';') SEPARATOR ' ')\n\
             INTO @sql FROM mysql.user WHERE User='root';\n\
             PREPARE stmt FROM @sql;\n\
             EXECUTE stmt;\n\
             DEALLOCATE PREPARE stmt;\n\
             -- 统计受影响的行数\n\
             SELECT COUNT(*) AS affected_rows FROM mysql.user WHERE User='root';\n\
             FLUSH PRIVILEGES;",
            escaped
        )
    } else if version.starts_with("5.7") {
        // MySQL 5.7
        format!(
            "-- MySQL 5.7 密码修改方式\n\
             UPDATE mysql.user SET authentication_string=PASSWORD('{}'), plugin='mysql_native_password' WHERE User='root';\n\
             SELECT ROW_COUNT() AS affected_rows;\n\
             FLUSH PRIVILEGES;",
            escaped
        )
    } else {
        // MySQL 5.6 及以下
        format!(
            "-- MySQL 5.6 及以下密码修改方式\n\
             UPDATE mysql.user SET Password=PASSWORD('{}'), plugin='mysql_native_password' WHERE User='root';\n\
             SELECT ROW_COUNT() AS affected_rows;\n\
             FLUSH PRIVILEGES;",
            escaped
        )
    }
}

/// 构建简单直接的 ALTER USER 方式（仅修改存在的用户）
pub fn build_simple_alter_sql(version: &str, new_password: &str, host: &str) -> Result<String, String> {
    validate_mysql_host(host)?;
    let escaped = escape_mysql_password(new_password);

    if is_mysql_8_or_higher(version) {
        Ok(format!(
            "ALTER USER 'root'@'{}' IDENTIFIED BY '{}'; FLUSH PRIVILEGES;",
            host, escaped
        ))
    } else {
        Ok(format!(
            "SET PASSWORD FOR 'root'@'{}' = PASSWORD('{}'); FLUSH PRIVILEGES;",
            host, escaped
        ))
    }
}

/// 验证 MySQL 主机名是否安全（仅允许主机名合法字符）
pub fn validate_mysql_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("主机名不能为空".into());
    }
    if host.len() > 253 {
        return Err("主机名过长".into());
    }
    if !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == '%') {
        return Err(format!("主机名包含非法字符: {}", host));
    }
    Ok(())
}

fn parse_affected_rows(stdout: &str) -> Option<u64> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("affected_rows") {
            continue;
        }
        if let Ok(count) = trimmed.parse::<u64>() {
            return Some(count);
        }
    }
    None
}

async fn start_mysql_service(app_handle: &AppHandle, service_name: &str) -> Result<(), String> {
    logger::info(app_handle, &format!("正在重新启动 MySQL 服务: {}", service_name));
    let result = process_manager::execute_command("net", &["start", service_name]).await;
    match result {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::warn(app_handle, &format!("启动服务命令返回非零退出码: {}", output.exit_code));
                logger::warn(app_handle, &format!("启动服务输出: {}", output.stdout));
                logger::warn(app_handle, &format!("启动服务错误: {}", output.stderr));
            } else {
                logger::info(app_handle, "启动服务命令执行成功");
            }
        }
        Err(e) => {
            logger::warn(app_handle, &format!("启动服务命令执行异常: {}", e));
        }
    }
    Ok(())
}

async fn test_mysql_connection(
    app_handle: &AppHandle, 
    mysql_path: &PathBuf, 
    password: &str,
    port: Option<u16>,
) -> Result<bool, String> {
    logger::info(app_handle, "正在测试 MySQL 连接...");
    let test_sql = "SELECT 1;";
    logger::info(app_handle, &format!("使用的 MySQL 程序路径: {:?}", mysql_path));
    if let Some(p) = port {
        logger::info(app_handle, &format!("使用端口: {}", p));
    }
    
    // 使用临时配置文件代替 -p 参数，避免密码泄露
    let config_guard = create_temp_mysql_config(password).await?;
    
    // 增加重试机制
    let max_retries = 5;
    let mut retry_count = 0;
    
    while retry_count < max_retries {
        retry_count += 1;
        logger::info(app_handle, &format!("连接测试尝试 {}/{}...", retry_count, max_retries));
        
        let mut args: Vec<String> = vec![
            format!("--defaults-file={}", config_guard.path().display()),
            "-e".to_string(),
            test_sql.to_string(),
        ];
        if let Some(p) = port {
            args.push("-h127.0.0.1".to_string());
            args.push("-P".to_string());
            args.push(p.to_string());
        }
        
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        
        let mysql_path_str = match mysql_path.to_str() {
            Some(s) => s,
            None => {
                logger::error(app_handle, "MySQL 路径无效");
                return Err("MySQL 路径无效".to_string());
            }
        };
        
        let result = process_manager::execute_command(
            mysql_path_str,
            &args_ref
        ).await;
        
        match result {
            Ok(output) => {
                logger::info(app_handle, &format!("命令执行退出码: {}", output.exit_code));
                if !output.stdout.is_empty() {
                    logger::info(app_handle, &format!("标准输出: {}", output.stdout));
                }
                if !output.stderr.is_empty() {
                    logger::warn(app_handle, &format!("错误输出: {}", output.stderr));
                }
                
                if output.exit_code == 0 {
                    logger::info(app_handle, "MySQL 连接测试成功！");
                    return Ok(true);
                } else {
                    logger::error(app_handle, &format!("MySQL 连接测试失败，退出码: {}", output.exit_code));
                    if retry_count < max_retries {
                        logger::info(app_handle, "等待 3 秒后重试...");
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    }
                }
            }
            Err(e) => {
                logger::error(app_handle, &format!("MySQL 连接测试异常: {}", e));
                if retry_count < max_retries {
                    logger::info(app_handle, "等待 3 秒后重试...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    }
    
    logger::error(app_handle, &format!("已重试 {} 次，全部失败", max_retries));
    Ok(false)
}

pub async fn reset_mysql_password(
    app_handle: AppHandle,
    new_password: String,
    selected_instance: Option<types::MySQLInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    // 首先验证新密码的安全性
    validate_password_strength(&new_password)?;

    logger::info(&app_handle, "开始自动重置 MySQL 密码");

    let mysql_info = detector::detect_all_mysql(Some(&app_handle)).await?;

    let instance = crate::detector_base::select_valid_instance(
        selected_instance.as_ref(),
        &mysql_info.instances,
        "MySQL",
    )?;

    let service_name = match &instance.service_name {
        Some(name) => name,
        None => {
            logger::error(&app_handle, "所选实例没有服务名，无法重置密码");
            return Err("所选实例没有服务名，无法重置密码".to_string());
        }
    };
    let version = &instance.version;
    let port = resolve_port(instance, override_port);

    logger::info(&app_handle, &format!("使用的 MySQL 实例: 版本 {}, 服务 {}, 路径 {}",
        version, service_name, instance.path));

    let mysql_path = PathBuf::from(&instance.path).join("mysql.exe");
    let mysqld_path = PathBuf::from(&instance.path).join("mysqld.exe");

    if !mysqld_path.exists() {
        logger::error(&app_handle, &format!("未找到 mysqld.exe: {:?}", mysqld_path));
        return Err(format!("未找到 mysqld.exe: {:?}", mysqld_path));
    }
    if !mysql_path.exists() {
        logger::error(&app_handle, &format!("未找到 mysql.exe: {:?}", mysql_path));
        return Err(format!("未找到 mysql.exe: {:?}", mysql_path));
    }

    stop_mysql_service(&app_handle, service_name).await?;
    kill_mysqld_processes(&app_handle, &instance.path).await?;

    logger::info(&app_handle, "正在以无授权模式启动 MySQL...");
    let config_file = detector::get_mysql_config_file(Some(service_name), &instance.path).await;
    // 高风险操作前将 my.ini 备份到下载目录/{版本号}/ 留底（重置不修改 my.ini，但停服/起无授权实例有风险）
    if let Some(config_path) = &config_file {
        let backup = crate::download_control::backup_config_to_download_dir(
            config_path,
            version,
        )
        .await;
        if let Some(p) = backup {
            logger::info(&app_handle, &format!("my.ini 已备份至: {}", p.display()));
        }
    }
    let mut startup_args: Vec<String> = Vec::new();
    if let Some(config_path) = &config_file {
        let config_arg = format!(r#"--defaults-file={}"#, config_path.display());
        startup_args.push(config_arg);
    } else {
        logger::warn(&app_handle, "未找到 my.ini 配置文件，可能连接到错误的数据目录");
    }
    startup_args.push("--skip-grant-tables".to_string());
    startup_args.push("--shared-memory".to_string());
    startup_args.push("--skip-networking".to_string());

    let mut cmd = tokio::process::Command::new(&mysqld_path);
    cmd.args(&startup_args);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    // 在 Windows 上隐藏控制台窗口，避免闪烁
    #[cfg(target_os = "windows")]
    hide_console_window(&mut cmd);
    
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            logger::error(&app_handle, &format!("启动 MySQL 失败: {}", e));
            // 服务已停止但临时实例未起来：立即恢复原服务
            restore_service_after_failure(&app_handle, service_name, &instance.path, None).await;
            return Err(format!("启动 MySQL 失败: {}", e));
        }
    };

    logger::info(&app_handle, "等待 MySQL 启动 (最多15秒)...");
    let mut started = false;
    for i in 1..=15u64 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        match child.try_wait() {
            Ok(Some(status)) => {
                logger::error(&app_handle, &format!("MySQL 进程在第 {} 秒意外退出，退出码: {:?}", i, status.code()));
                let stderr_text = if let Some(mut stderr) = child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let mut buf = String::new();
                    let _ = stderr.read_to_string(&mut buf).await;
                    buf
                } else {
                    String::new()
                };
                if !stderr_text.is_empty() {
                    logger::error(&app_handle, &format!("MySQL 错误输出:\n{}", stderr_text));
                }
                // 无授权实例启动即失败：恢复原服务，避免服务停摆
                restore_service_after_failure(&app_handle, service_name, &instance.path, Some(&mut child)).await;
                return Err(format!("MySQL 进程在启动后第 {} 秒退出，请查看上方错误输出", i));
            }
            Ok(None) => {
                if i >= 6 {
                    logger::info(&app_handle, &format!("MySQL 进程已运行 {} 秒，判定为启动成功", i));
                    started = true;
                    break;
                }
            }
            Err(e) => {
                logger::warn(&app_handle, &format!("检查进程状态失败: {}", e));
                break;
            }
        }
    }
    if !started {
        logger::warn(&app_handle, "等待超时，尝试继续执行...");
    }

    logger::info(&app_handle, "正在连接并修改密码...");

    let query_users_sql = "SELECT User, Host, LENGTH(User) AS user_len FROM mysql.user;";
    let query_result = process_manager::execute_command(
        mysql_path.to_str().unwrap_or(""),
        &["-u", "root", "--protocol=memory", "-e", query_users_sql]
    ).await;

    if let Ok(output) = &query_result {
        if output.exit_code != 0 {
            logger::warn(&app_handle, &format!("查询用户失败:\n{}", output.stderr));
        }
    }
    
    if is_mysql_9_or_higher(version) {
        logger::info(&app_handle, "检测到 MySQL 9.0+，mysql_native_password 已移除，使用 caching_sha2_password + ALTER USER 方式...");
    } else if is_mysql_8_or_higher(version) {
        logger::info(&app_handle, "检测到 MySQL 8.0+，使用清空认证信息后 ALTER USER 方式...");
    } else if version.starts_with("5.7") {
        logger::info(&app_handle, "检测到 MySQL 5.7，使用 5.7 专用重置方式...");
    } else {
        logger::info(&app_handle, &format!("检测到 MySQL 版本 {}，使用 5.6 及以下重置方式...", version));
    }
    let full_sql = build_password_reset_sql(version, &new_password);

    let mysql_path_str = match mysql_path.to_str() {
        Some(s) => s,
        None => {
            logger::warn(&app_handle, "MySQL 路径无法转换为 UTF-8 字符串");
            // 无授权实例正在运行：恢复原服务后再返回错误
            restore_service_after_failure(&app_handle, service_name, &instance.path, Some(&mut child)).await;
            return Err("MySQL 路径无效".into());
        }
    };
    
    let result = process_manager::execute_command(
        mysql_path_str,
        &["-u", "root", "--protocol=memory", "-e", &full_sql]
    ).await;
    
    let mut sql_success = false;
    match result {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::warn(&app_handle, &format!("SQL 执行警告: {}", output.stderr));
            } else {
                let affected_rows = parse_affected_rows(&output.stdout);
                if let Some(count) = affected_rows {
                    if count > 0 || is_mysql_8_or_higher(version) {
                        sql_success = true;
                        logger::info(&app_handle, "SQL 执行成功！");
                    } else {
                        logger::error(&app_handle, "密码更新未影响任何 root 用户，重置失败");
                    }
                } else {
                    // MySQL 8.x 的 ALTER USER 不输出 ROW_COUNT，无法据此判断，保持宽松成功；
                    // 5.6/5.7 路径含 SELECT ROW_COUNT()，输出异常意味着实际失败，不应误判成功
                    if is_mysql_8_or_higher(version) {
                        logger::warn(&app_handle, "无法解析受影响行数（MySQL 8.x+ ALTER USER 不输出行数），保持宽松判断");
                        sql_success = true;
                    } else {
                        logger::error(&app_handle, "无法解析受影响行数，且非 MySQL 8.x+，判定重置失败");
                    }
                }
            }
            if !output.stderr.is_empty() && output.exit_code == 0 {
                logger::warn(&app_handle, &format!("SQL 错误输出: {}", output.stderr));
            }
        }
        Err(e) => {
            logger::warn(&app_handle, &format!("SQL 执行错误: {}", e));
        }
    }

    // 在无授权模式下验证密码是否设置成功
    if sql_success {
        logger::info(&app_handle, "正在无授权模式下验证密码设置...");
        // 对于老版本检查password字段，对于5.7+检查authentication_string
        let verify_sql = if version.starts_with("5.7") || is_mysql_8_or_higher(version) {
            "SELECT User, Host, LEFT(authentication_string, 10) AS pass_prefix FROM mysql.user WHERE User='root';"
        } else {
            "SELECT User, Host, LEFT(Password, 10) AS pass_prefix FROM mysql.user WHERE User='root';"
        };
        
        let verify_result = process_manager::execute_command(
            mysql_path_str,
            &["-u", "root", "--protocol=memory", "-e", verify_sql]
        ).await;
        
        match verify_result {
            Ok(output) => {
                if output.exit_code != 0 {
                    logger::warn(&app_handle, &format!("验证查询警告:\n{}", output.stderr));
                }
                if !output.stderr.is_empty() && output.exit_code == 0 {
                    logger::warn(&app_handle, &format!("验证查询错误输出:\n{}", output.stderr));
                }
            }
            Err(e) => {
                logger::warn(&app_handle, &format!("验证查询执行错误: {}", e));
            }
        }
    }

    logger::info(&app_handle, "正在停止无授权模式的 MySQL...");
    match child.kill().await {
        Ok(_) => logger::info(&app_handle, "已发送停止信号到 MySQL 进程"),
        Err(e) => logger::warn(&app_handle, &format!("停止 MySQL 进程时出错: {}", e)),
    }
    // 精准清理该实例残留进程并轮询等待退出（kill_mysqld_processes 内部已包含等待）
    kill_mysqld_processes(&app_handle, &instance.path).await?;
    start_mysql_service(&app_handle, service_name).await?;

    // 轮询等待服务进入"启动"状态（最多 40 秒），替代固定 sleep
    logger::info(&app_handle, "等待 MySQL 服务启动...");
    let service_ready = wait_for_service_state(&app_handle, service_name, true, 40).await;
    logger::info(
        &app_handle,
        if service_ready {
            "MySQL 服务已进入启动状态"
        } else {
            "MySQL 服务启动等待超时，仍将尝试验证连接"
        },
    );

    // 通过 TCP 连接轮询确认端口就绪（替代 netstat 文本匹配，避免误判）
    let port_to_check = port.unwrap_or(3306);
    logger::info(&app_handle, &format!("等待 MySQL 端口 ({}) 就绪...", port_to_check));
    let port_ready = wait_for_port_ready(&app_handle, port_to_check, 40).await;
    if port_ready {
        logger::info(&app_handle, &format!("MySQL 端口 {} 已就绪", port_to_check));
    } else {
        logger::warn(&app_handle, &format!("MySQL 端口 {} 在超时时间内未就绪", port_to_check));
    }

    logger::info(&app_handle, "开始连接测试...");
    match test_mysql_connection(&app_handle, &mysql_path, &new_password, port).await {
        Ok(true) => {
            logger::info(&app_handle, "========================================");
            logger::info(&app_handle, "密码重置成功！连接测试通过！");
            logger::info(&app_handle, "========================================");
            Ok("密码重置成功，连接测试通过！".to_string())
        }
        Ok(false) => {
            if sql_success {
                logger::warn(&app_handle, "========================================");
                logger::warn(&app_handle, "连接测试失败，但密码可能已成功设置！");
                logger::warn(&app_handle, "========================================");
                logger::warn(&app_handle, "建议您手动尝试用新密码连接 MySQL");
                Ok("密码可能已成功设置！虽然连接测试失败，但 SQL 命令执行成功。请尝试手动连接。".to_string())
            } else {
                logger::error(&app_handle, "========================================");
                logger::error(&app_handle, "密码重置失败！连接测试未通过");
                logger::error(&app_handle, "========================================");
                logger::error(&app_handle, "可能的原因：");
                logger::error(&app_handle, "  1. MySQL 服务可能没有完全启动");
                logger::error(&app_handle, "  2. 密码更新可能没有成功");
                logger::error(&app_handle, "  3. 用户权限配置可能有问题");
                Err("密码重置失败，连接测试未通过，请查看日志获取详细信息".to_string())
            }
        }
        Err(e) => {
            if sql_success {
                logger::warn(&app_handle, "========================================");
                logger::warn(&app_handle, "连接测试异常，但密码可能已成功设置！");
                logger::warn(&app_handle, "========================================");
                logger::warn(&app_handle, &format!("连接测试异常: {}", e));
                logger::warn(&app_handle, "建议您手动尝试用新密码连接 MySQL");
                Ok("密码可能已成功设置！虽然连接测试异常，但 SQL 命令执行成功。请尝试手动连接。".to_string())
            } else {
                logger::error(&app_handle, "========================================");
                logger::error(&app_handle, &format!("密码重置失败！连接测试异常: {}", e));
                logger::error(&app_handle, "========================================");
                Err(format!("密码重置失败，连接测试异常: {}", e))
            }
        }
    }
}

pub async fn change_mysql_password(
    app_handle: AppHandle,
    old_password: String,
    new_password: String,
    selected_instance: Option<types::MySQLInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    // 验证新密码的安全性
    validate_password_strength(&new_password)?;
    
    let mysql_info = detector::detect_all_mysql(Some(&app_handle)).await?;

    let instance = if let Some(sel_inst) = &selected_instance {
        sel_inst
    } else {
        mysql_info
            .instances
            .iter()
            .find(|inst| !inst.path.is_empty())
            .ok_or_else(|| "未找到 MySQL 安装路径".to_string())?
    };
    logger::info(&app_handle, &format!("使用实例: 版本 {}, 路径 {}", instance.version, instance.path));

    let mysql_path = PathBuf::from(&instance.path).join("mysql.exe");
    let port = resolve_port(instance, override_port);
    if !mysql_path.exists() {
        return Err("mysql.exe 未找到".to_string());
    }

    logger::info(&app_handle, "========== 开始修改密码 ==========");
    if let Some(p) = port {
        logger::info(&app_handle, &format!("使用端口: {} ({})", p, if override_port.is_some() { "手动指定" } else { "自动检测" }));
    }

    let version = &instance.version;
    let mysql_path_str = match mysql_path.to_str() {
        Some(s) => s,
        None => {
            return Err("MySQL 路径无效".to_string());
        }
    };

    // 使用临时配置文件代替 -p 参数，避免密码泄露
    let config_guard = create_temp_mysql_config(&old_password).await?;

    // 构建基础参数
    let mut base_args: Vec<String> = vec![
        format!("--defaults-file={}", config_guard.path().display()),
    ];
    if let Some(p) = port {
        base_args.push("-h127.0.0.1".to_string());
        base_args.push("-P".to_string());
        base_args.push(p.to_string());
    }
    
    // 首先尝试用旧密码连接，验证旧密码是否正确
    logger::info(&app_handle, "正在验证旧密码是否正确...");
    let test_sql = "SELECT 1;";
    let mut test_args = base_args.clone();
    test_args.push("-e".to_string());
    test_args.push(test_sql.to_string());
    
    let test_args_ref: Vec<&str> = test_args.iter().map(|s| s.as_str()).collect();
    let test_result = process_manager::execute_command(mysql_path_str, &test_args_ref).await;
    
    match &test_result {
        Ok(output) if output.exit_code != 0 => {
            // 检查是否是访问被拒绝的错误
            let is_access_denied = output.stderr.contains("Access denied") || 
                                   output.stderr.contains("ERROR 1045");
            if is_access_denied {
                logger::error(&app_handle, "❌ 旧密码不正确！请检查你输入的旧密码是否正确。");
                return Err("旧密码不正确！请确认你输入的旧密码是否正确，或者使用密码重置功能。".to_string());
            }
        }
        Ok(output) if output.exit_code == 0 => {
            logger::info(&app_handle, "✅ 旧密码验证成功！");
        }
        Err(e) => {
            logger::warn(&app_handle, &format!("验证过程异常: {}", e));
        }
        _ => {}
    }
    
    // 使用最简单直接的方法：先尝试只修改 localhost
    logger::info(&app_handle, &format!("检测到 MySQL 版本 {}, 使用简单直接的方式修改密码...", version));
    
    let simple_sql = build_simple_alter_sql(version, &new_password, "localhost")?;
    let masked_simple_sql = simple_sql.replace(&escape_mysql_password(&new_password), "***");
    logger::info(&app_handle, &format!("执行 SQL (密码已掩码): {}", masked_simple_sql));
    
    // 执行修改
    let mut modify_args = base_args.clone();
    modify_args.push("-e".to_string());
    modify_args.push(simple_sql.clone());
    
    let modify_args_ref: Vec<&str> = modify_args.iter().map(|s| s.as_str()).collect();
    
    let result = process_manager::execute_command(mysql_path_str, &modify_args_ref).await;

    match result {
        Ok(output) if output.exit_code == 0 => {
            logger::info(&app_handle, "✅ 密码修改命令执行成功！");
            
            logger::info(&app_handle, "正在测试连接...");
            
            let test_result = test_mysql_connection(&app_handle, &mysql_path, &new_password, port).await;
            
            match test_result {
                Ok(true) => {
                    logger::info(&app_handle, "========== 密码修改成功！连接测试通过！ ==========");
                    Ok("密码修改成功，连接测试通过！".to_string())
                }
                Ok(false) => {
                    logger::warn(&app_handle, "连接测试失败，但密码可能已修改成功！");
                    Ok("密码可能已成功修改！虽然连接测试失败，但 SQL 命令执行成功。请尝试手动连接。".to_string())
                }
                Err(e) => {
                    logger::warn(&app_handle, &format!("连接测试异常，但密码可能已修改成功: {}", e));
                    Ok("密码可能已成功修改！虽然连接测试异常，但 SQL 命令执行成功。请尝试手动连接。".to_string())
                }
            }
        }
        Ok(output) => {
            // 如果修改 localhost 失败，尝试更复杂的方案
            logger::warn(&app_handle, &format!("简单方案失败: {}", output.stderr));
            logger::info(&app_handle, "尝试备用方案...");
            
            // 备用方案：尝试使用动态生成的 SQL（MySQL 8.0+，含 9.0+）
            if is_mysql_8_or_higher(version) {
                logger::info(&app_handle, "尝试 MySQL 8.0+（含 9.0+）的动态 SQL 方案...");
                let fallback_sql = build_safe_change_password_sql(version, &new_password);
                let masked_fallback_sql = fallback_sql.replace(&escape_mysql_password(&new_password), "***");
                logger::info(&app_handle, &format!("执行 SQL (密码已掩码): {}", masked_fallback_sql));
                
                let mut fallback_args = base_args.clone();
                fallback_args.push("-e".to_string());
                fallback_args.push(fallback_sql.clone());
                
                let fallback_args_ref: Vec<&str> = fallback_args.iter().map(|s| s.as_str()).collect();
                
                let fallback_result = process_manager::execute_command(mysql_path_str, &fallback_args_ref).await;
                
                match fallback_result {
                    Ok(fb_output) if fb_output.exit_code == 0 => {
                        logger::info(&app_handle, "备用方案成功！正在测试连接...");
                        match test_mysql_connection(&app_handle, &mysql_path, &new_password, port).await {
                            Ok(true) => {
                                logger::info(&app_handle, "密码修改成功（备用方案）！连接测试通过！");
                                Ok("密码修改成功（备用方案），连接测试通过！".to_string())
                            }
                            _ => {
                                Ok("密码可能已成功修改！请尝试手动连接。".to_string())
                            }
                        }
                    }
                    _ => {
                        // 如果备用方案也失败，给出友好提示
                        logger::error(&app_handle, "所有修改密码的方案都失败了！");
                        logger::info(&app_handle, "💡 建议：如果旧密码忘记了，请使用密码重置功能！");
                        Err("密码修改失败！请确认旧密码是否正确，或者尝试使用密码重置功能。".to_string())
                    }
                }
            } else {
                // 对于旧版本，尝试 UPDATE 方式
                logger::info(&app_handle, "尝试 UPDATE 方式修改密码...");
                let escaped = escape_mysql_password(&new_password);
                let old_version_sql = if version.starts_with("5.7") {
                    format!("UPDATE mysql.user SET authentication_string=PASSWORD('{}') WHERE User='root' AND Host='localhost'; FLUSH PRIVILEGES;", escaped)
                } else {
                    format!("UPDATE mysql.user SET Password=PASSWORD('{}') WHERE User='root' AND Host='localhost'; FLUSH PRIVILEGES;", escaped)
                };
                
                let masked_old_sql = old_version_sql.replace(&escaped, "***");
                logger::info(&app_handle, &format!("执行 SQL (密码已掩码): {}", masked_old_sql));
                
                let mut old_args = base_args.clone();
                old_args.push("-e".to_string());
                old_args.push(old_version_sql.clone());
                
                let old_args_ref: Vec<&str> = old_args.iter().map(|s| s.as_str()).collect();
                
                let old_result = process_manager::execute_command(mysql_path_str, &old_args_ref).await;
                
                match old_result {
                    Ok(old_output) if old_output.exit_code == 0 => {
                        logger::info(&app_handle, "UPDATE 方式成功！正在测试连接...");
                        match test_mysql_connection(&app_handle, &mysql_path, &new_password, port).await {
                            Ok(true) => {
                                logger::info(&app_handle, "密码修改成功！连接测试通过！");
                                Ok("密码修改成功，连接测试通过！".to_string())
                            }
                            _ => {
                                Ok("密码可能已成功修改！请尝试手动连接。".to_string())
                            }
                        }
                    }
                    _ => {
                        logger::error(&app_handle, "所有修改密码的方案都失败了！");
                        logger::info(&app_handle, "💡 建议：如果旧密码忘记了，请使用密码重置功能！");
                        Err(format!("密码修改失败！错误信息: {}", output.stderr))
                    }
                }
            }
        }
        Err(e) => {
            logger::error(&app_handle, &format!("执行异常: {}", e));
            Err(format!("执行异常: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_mysql_password_handles_special_chars() {
        assert_eq!(escape_mysql_password(r"a\b"), r"a\\b");
        assert_eq!(escape_mysql_password("it's"), "it''s");
        assert_eq!(escape_mysql_password("\"quote\""), r#"\"quote\""#);
        assert_eq!(escape_mysql_password("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_mysql_password("tab\tsep"), "tab\\tsep");
        assert_eq!(escape_mysql_password("\x00null"), "\\0null");
        assert_eq!(escape_mysql_password("\x1actrlz"), "\\Zctrlz");
    }

    #[test]
    fn validate_password_strength_rejects_short_password() {
        assert!(validate_password_strength("123").is_err());
        assert!(validate_password_strength("12345").is_err());
    }

    #[test]
    fn validate_password_strength_accepts_normal_passwords() {
        assert!(validate_password_strength("123456").is_ok());
        assert!(validate_password_strength("Password1").is_ok());
        assert!(validate_password_strength("SecurePass123!").is_ok());
        assert!(validate_password_strength("my_pass1").is_ok());
        assert!(validate_password_strength("ABCdef12").is_ok());
    }

    #[test]
    fn build_password_reset_sql_for_mysql8_includes_root_percent() {
        let sql = build_password_reset_sql("8.0.36", "newpass");
        assert!(sql.contains("root'@'%'"));
        assert!(sql.contains("newpass"));
    }

    #[test]
    fn build_safe_change_password_sql_for_mysql57_uses_update() {
        let sql = build_safe_change_password_sql("5.7.44", "secret");
        assert!(sql.contains("UPDATE mysql.user"));
        assert!(sql.contains("authentication_string"));
        assert!(sql.contains("secret"));
    }

    #[test]
    fn build_safe_change_password_sql_for_mysql8_uses_alter_user() {
        let sql = build_safe_change_password_sql("8.0.36", "secret");
        assert!(sql.contains("ALTER USER"));
        assert!(sql.contains("authentication_string") || sql.contains("IDENTIFIED BY"));
        assert!(sql.contains("secret"));
    }
    
    #[test]
    fn build_password_reset_sql_escapes_special_chars() {
        let sql = build_password_reset_sql("8.0.36", "pass'with\"special");
        assert!(sql.contains("pass''with\\\"special"));
    }

    #[test]
    fn is_mysql_8_or_higher_detects_version_correctly() {
        assert!(is_mysql_8_or_higher("8.0.36"));
        assert!(is_mysql_8_or_higher("8.4.0"));
        assert!(is_mysql_8_or_higher("9.0.0"));
        assert!(is_mysql_8_or_higher("9.1.2"));
        assert!(!is_mysql_8_or_higher("5.7.44"));
        assert!(!is_mysql_8_or_higher("5.6.51"));
        assert!(!is_mysql_8_or_higher("unknown"));
    }

    #[test]
    fn is_mysql_9_or_higher_detects_version_correctly() {
        assert!(!is_mysql_9_or_higher("8.0.36"));
        assert!(!is_mysql_9_or_higher("8.4.0"));
        assert!(is_mysql_9_or_higher("9.0.0"));
        assert!(is_mysql_9_or_higher("9.1.2"));
        assert!(is_mysql_9_or_higher("9.5.0"));
        assert!(!is_mysql_9_or_higher("5.7.44"));
        assert!(!is_mysql_9_or_higher("unknown"));
    }

    #[test]
    fn build_password_reset_sql_for_mysql9_uses_caching_sha2() {
        let sql = build_password_reset_sql("9.0.1", "newpass");
        assert!(sql.contains("ALTER USER"));
        assert!(sql.contains("caching_sha2_password"));
        assert!(sql.contains("newpass"));
        // 9.0+ must NOT use mysql_native_password or PASSWORD() function
        assert!(!sql.contains("mysql_native_password"));
        assert!(!sql.contains("PASSWORD("));
    }

    #[test]
    fn build_password_reset_sql_for_mysql8_uses_native_password() {
        let sql = build_password_reset_sql("8.0.36", "newpass");
        assert!(sql.contains("ALTER USER"));
        assert!(sql.contains("mysql_native_password"));
        assert!(sql.contains("newpass"));
    }

    #[test]
    fn build_safe_change_password_sql_for_mysql9_uses_alter_user() {
        let sql = build_safe_change_password_sql("9.1.0", "secret");
        assert!(sql.contains("ALTER USER"));
        assert!(!sql.contains("PASSWORD("));
    }

    #[test]
    fn build_simple_alter_sql_for_mysql9_uses_alter_user() {
        let sql = build_simple_alter_sql("9.0.0", "secret", "localhost").unwrap();
        assert!(sql.contains("ALTER USER 'root'@'localhost' IDENTIFIED BY"));
        assert!(!sql.contains("PASSWORD("));
    }
}
