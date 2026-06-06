use super::super::{logger, process_manager, types};
use super::detector;
use std::path::PathBuf;
use tauri::AppHandle;

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
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    Ok(())
}

async fn kill_mysqld_processes(app_handle: &AppHandle) -> Result<(), String> {
    logger::info(app_handle, "正在终止所有 MySQL 进程...");
    let _ = process_manager::execute_command("taskkill", &["/F", "/IM", "mysqld.exe"]).await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    Ok(())
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

/// 验证密码是否符合基本安全要求
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 4 {
        return Err("密码长度至少需要 4 个字符".to_string());
    }
    
    Ok(())
}

fn resolve_port(instance: &types::MySQLInstance, override_port: Option<u16>) -> Option<u16> {
    override_port.or(instance.port)
}

pub fn build_password_reset_sql(version: &str, new_password: &str) -> String {
    let escaped_password = escape_mysql_password(new_password);

    if version.starts_with("8.") {
        vec![
            format!(
                "UPDATE mysql.user SET authentication_string='', plugin='mysql_native_password' WHERE User='root';"
            ),
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
        vec![
            format!(
                "UPDATE mysql.user SET authentication_string=PASSWORD('{}'), plugin='mysql_native_password', password_expired='N' WHERE User='root';",
                escaped_password
            ),
            "SELECT ROW_COUNT() AS affected_rows;".to_string(),
            "FLUSH PRIVILEGES;".to_string(),
        ]
        .join(" ")
    } else {
        vec![
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

/// 构建修改密码的SQL脚本 - 首先查询存在的用户，再修改
/// 返回查询用户SQL和修改密码SQL
pub fn build_change_password_sql(version: &str, new_password: &str) -> (String, String) {
    let escaped = escape_mysql_password(new_password);
    
    // 首先查询存在的用户
    let query_users_sql = "SELECT User, Host FROM mysql.user WHERE User='root';".to_string();
    
    // 根据版本构建修改密码的SQL
    let modify_sql = if version.starts_with("8.") {
        // MySQL 8.0: 使用 ALTER USER，但通过 UPDATE 方式，避免用户不存在的问题
        format!(
            "UPDATE mysql.user SET authentication_string=PASSWORD('{}'), plugin='mysql_native_password' WHERE User='root'; \
             FLUSH PRIVILEGES;",
            escaped
        )
    } else if version.starts_with("5.7") {
        // MySQL 5.7: 使用 UPDATE 方式，更安全
        format!(
            "UPDATE mysql.user SET authentication_string=PASSWORD('{}'), plugin='mysql_native_password' WHERE User='root'; \
             FLUSH PRIVILEGES;",
            escaped
        )
    } else {
        // MySQL 5.6 及以下: 使用 UPDATE 方式
        format!(
            "UPDATE mysql.user SET Password=PASSWORD('{}'), plugin='mysql_native_password' WHERE User='root'; \
             FLUSH PRIVILEGES;",
            escaped
        )
    };
    
    (query_users_sql, modify_sql)
}

/// 构建容错版修改密码SQL - 逐个尝试修改，失败不影响后续
pub fn build_safe_change_password_sql(version: &str, new_password: &str) -> String {
    let escaped = escape_mysql_password(new_password);
    
    if version.starts_with("8.") {
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
pub fn build_simple_alter_sql(version: &str, new_password: &str, host: &str) -> String {
    let escaped = escape_mysql_password(new_password);
    
    if version.starts_with("8.") {
        format!(
            "ALTER USER 'root'@'{}' IDENTIFIED BY '{}'; FLUSH PRIVILEGES;",
            host, escaped
        )
    } else {
        format!(
            "SET PASSWORD FOR 'root'@'{}' = PASSWORD('{}'); FLUSH PRIVILEGES;",
            host, escaped
        )
    }
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
    
    // 增加重试机制
    let max_retries = 5;
    let mut retry_count = 0;
    
    while retry_count < max_retries {
        retry_count += 1;
        logger::info(app_handle, &format!("连接测试尝试 {}/{}...", retry_count, max_retries));
        
        let p_arg = format!("-p{}", password);
        let mut args: Vec<String> = vec![
            "-u".to_string(),
            "root".to_string(),
            p_arg,
            "-e".to_string(),
            test_sql.to_string(),
        ];
        if let Some(p) = port {
            args.insert(2, "-h127.0.0.1".to_string());
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
    
    logger::info(&app_handle, "========================================");
    logger::info(&app_handle, "开始自动重置 MySQL 密码");
    logger::info(&app_handle, "========================================");

    let mysql_info = detector::detect_all_mysql(Some(&app_handle)).await;
    logger::info(&app_handle, &format!("检测到 {} 个 MySQL 实例", mysql_info.instances.len()));
    
    let instance: &types::MySQLInstance;
    if let Some(sel_inst) = &selected_instance {
        instance = sel_inst;
        logger::info(&app_handle, &format!("使用用户选择的实例: 版本 {}, 服务 {:?}, 路径 {}", 
            sel_inst.version, sel_inst.service_name, sel_inst.path));
    } else {
        let valid_instance = mysql_info.instances.iter()
            .find(|inst| !inst.path.is_empty() && inst.service_name.is_some());

        if valid_instance.is_none() {
            logger::error(&app_handle, "未找到有效的 MySQL 实例（需要有安装路径和服务名）");
            logger::error(&app_handle, "检测到的实例详情：");
            for (idx, inst) in mysql_info.instances.iter().enumerate() {
                logger::error(&app_handle, &format!("  实例 {}: 路径={:?}, 服务名={:?}", idx+1, inst.path, inst.service_name));
            }
            return Err("未找到有效的 MySQL 实例（需要有安装路径和服务名）".to_string());
        }
        instance = valid_instance.unwrap();
    }

    let service_name = match &instance.service_name {
        Some(name) => name,
        None => {
            logger::error(&app_handle, "所选实例没有服务名，无法重置密码");
            return Err("所选实例没有服务名，无法重置密码".to_string());
        }
    };
    let version = &instance.version;
    let port = resolve_port(instance, override_port);
    if let Some(p) = port {
        logger::info(&app_handle, &format!("使用端口: {} ({})", p, if override_port.is_some() { "手动指定" } else { "自动检测" }));
    }

    logger::info(&app_handle, &format!("使用的 MySQL 实例: 版本 {}, 服务 {}, 路径 {}", 
        version, service_name, instance.path));

    let mysql_path = PathBuf::from(&instance.path).join("mysql.exe");
    let mysqld_path = PathBuf::from(&instance.path).join("mysqld.exe");

    logger::info(&app_handle, &format!("检查 MySQL 可执行文件: mysql.exe={:?}, mysqld.exe={:?}", 
        mysql_path.exists(), mysqld_path.exists()));

    if !mysqld_path.exists() {
        logger::error(&app_handle, &format!("未找到 mysqld.exe: {:?}", mysqld_path));
        return Err(format!("未找到 mysqld.exe: {:?}", mysqld_path));
    }
    if !mysql_path.exists() {
        logger::error(&app_handle, &format!("未找到 mysql.exe: {:?}", mysql_path));
        return Err(format!("未找到 mysql.exe: {:?}", mysql_path));
    }

    stop_mysql_service(&app_handle, service_name).await?;
    kill_mysqld_processes(&app_handle).await?;

    logger::info(&app_handle, "正在以无授权模式启动 MySQL...");
    let config_file = detector::get_mysql_config_file(Some(service_name), &instance.path).await;
    let mut startup_args: Vec<String> = Vec::new();
    if let Some(config_path) = &config_file {
        let config_arg = format!(r#"--defaults-file={}"#, config_path.display());
        logger::info(&app_handle, &format!("使用配置文件: {}", config_path.display()));
        startup_args.push(config_arg);
    } else {
        logger::warn(&app_handle, "未找到 my.ini 配置文件，可能连接到错误的数据目录");
    }
    startup_args.push("--skip-grant-tables".to_string());
    startup_args.push("--shared-memory".to_string());
    startup_args.push("--skip-networking".to_string());

    let args_str = startup_args.join(" ");
    logger::info(&app_handle, &format!("启动命令: mysqld {}", args_str));

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
    
    // 首先查询一下当前的用户情况（查看所有用户）
    logger::info(&app_handle, "正在查询当前MySQL所有用户信息...");
    let query_users_sql = "SELECT User, Host, LENGTH(User) AS user_len FROM mysql.user;";
    let query_result = process_manager::execute_command(
        mysql_path.to_str().unwrap(),
        &["-u", "root", "--protocol=memory", "-e", query_users_sql]
    ).await;
    
    if let Ok(output) = &query_result {
        if output.exit_code == 0 {
            logger::info(&app_handle, &format!("当前所有用户信息:\n{}", output.stdout));
        } else {
            logger::warn(&app_handle, &format!("查询用户失败:\n{}", output.stderr));
        }
    }
    
    if version.starts_with("8.") {
        logger::info(&app_handle, "检测到 MySQL 8.0+，使用清空认证信息后 ALTER USER 方式...");
    } else if version.starts_with("5.7") {
        logger::info(&app_handle, "检测到 MySQL 5.7，使用 5.7 专用重置方式...");
    } else {
        logger::info(&app_handle, &format!("检测到 MySQL 版本 {}，使用 5.6 及以下重置方式...", version));
    }
    let full_sql = build_password_reset_sql(version, &new_password);

    // 在日志中掩码密码，避免明文显示
    let masked_sql = full_sql.replace(&new_password, "***");
    logger::info(&app_handle, &format!("执行 SQL 脚本 (密码已掩码): {}", masked_sql));
    
    let mysql_path_str = match mysql_path.to_str() {
        Some(s) => s,
        None => {
            logger::warn(&app_handle, "MySQL 路径无法转换为 UTF-8 字符串");
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
            logger::info(&app_handle, &format!("SQL 执行退出码: {}", output.exit_code));
            if output.exit_code != 0 {
                logger::warn(&app_handle, &format!("SQL 执行警告: {}", output.stderr));
            } else {
                let affected_rows = parse_affected_rows(&output.stdout);
                if let Some(count) = affected_rows {
                    logger::info(&app_handle, &format!("密码更新影响行数: {}", count));
                    if count > 0 || version.starts_with("8.") {
                        sql_success = true;
                        logger::info(&app_handle, "SQL 执行成功！");
                    } else {
                        logger::error(&app_handle, "密码更新未影响任何 root 用户，重置失败");
                    }
                } else {
                    logger::warn(&app_handle, "无法解析受影响行数，将继续尝试后续步骤");
                    sql_success = true;
                }
            }
            if !output.stdout.is_empty() {
                logger::info(&app_handle, &format!("SQL 输出: {}", output.stdout));
            } else {
                logger::info(&app_handle, "SQL 输出: (空)");
            }
            if !output.stderr.is_empty() {
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
        let verify_sql = if version.starts_with("5.7") || version.starts_with("8.") {
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
                logger::info(&app_handle, &format!("验证查询退出码: {}", output.exit_code));
                if output.exit_code == 0 {
                    logger::info(&app_handle, &format!("用户和密码验证结果:\n{}", output.stdout));
                } else {
                    logger::warn(&app_handle, &format!("验证查询警告:\n{}", output.stderr));
                }
                if !output.stderr.is_empty() {
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
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    
    kill_mysqld_processes(&app_handle).await?;
    start_mysql_service(&app_handle, service_name).await?;

    logger::info(&app_handle, "等待 MySQL 服务完全启动 (10秒)...");
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    // 检查服务状态
    logger::info(&app_handle, "检查 MySQL 服务状态...");
    let service_status = detector::check_service_status(service_name).await;
    logger::info(&app_handle, &format!("MySQL 服务状态: {}", service_status));

    // 如果服务状态不是 "启动"，继续等待
    if service_status != "启动" {
        logger::warn(&app_handle, "MySQL 服务未完全启动，继续等待 10 秒...");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        
        let service_status2 = detector::check_service_status(service_name).await;
        logger::info(&app_handle, &format!("MySQL 服务状态 (第二次检查): {}", service_status2));
    }

    // 检查端口监听状态
    let port_to_check = port.unwrap_or(3306);
    logger::info(&app_handle, &format!("检查 MySQL 端口 ({}) 监听状态...", port_to_check));
    let netstat_result = process_manager::execute_command("netstat", &["-ano"]).await;
    match netstat_result {
        Ok(output) => {
            if output.exit_code == 0 {
                let port_str = format!(":{}", port_to_check);
                let has_port = output.stdout.contains(&port_str) || output.stdout.contains(&port_to_check.to_string());
                if has_port {
                    logger::info(&app_handle, &format!("MySQL 端口 {} 正在监听！", port_to_check));
                } else {
                    logger::warn(&app_handle, &format!("未检测到 MySQL 端口 {} 监听", port_to_check));
                    // 显示所有监听的端口...
                }
            }
        }
        Err(e) => {
            logger::warn(&app_handle, &format!("检查端口状态失败: {}", e));
        }
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
    
    let mysql_info = detector::detect_all_mysql(Some(&app_handle)).await;
    
    let instance: &types::MySQLInstance;
    if let Some(sel_inst) = &selected_instance {
        instance = sel_inst;
        logger::info(&app_handle, &format!("使用用户选择的实例: 版本 {}, 路径 {}", 
            sel_inst.version, sel_inst.path));
    } else {
        let valid_instance = mysql_info.instances.iter()
            .find(|inst| !inst.path.is_empty());
        
        match valid_instance {
            Some(inst) => instance = inst,
            None => {
                return Err("未找到 MySQL 安装路径".to_string());
            }
        }
    }

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
    let p_arg = format!("-p{}", old_password);

    // 构建基础参数
    let mut base_args: Vec<String> = vec![
        "-u".to_string(),
        "root".to_string(),
        p_arg,
    ];
    if let Some(p) = port {
        base_args.insert(2, "-h127.0.0.1".to_string());
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
            // 继续尝试，可能是网络或其他问题
        }
        _ => {}
    }
    
    // 使用最简单直接的方法：先尝试只修改 localhost
    logger::info(&app_handle, &format!("检测到 MySQL 版本 {}, 使用简单直接的方式修改密码...", version));
    
    let simple_sql = build_simple_alter_sql(version, &new_password, "localhost");
    let masked_simple_sql = simple_sql.replace(&new_password, "***");
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
            
            match test_mysql_connection(&app_handle, &mysql_path, &new_password, port).await {
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
            
            // 备用方案：尝试使用动态生成的 SQL（MySQL 8.0+）
            if version.starts_with("8.") {
                logger::info(&app_handle, "尝试 MySQL 8.0+ 的动态 SQL 方案...");
                let fallback_sql = build_safe_change_password_sql(version, &new_password);
                let masked_fallback_sql = fallback_sql.replace(&new_password, "***");
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
                
                let masked_old_sql = old_version_sql.replace(&new_password, "***");
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
        assert!(validate_password_strength("1234").is_ok());
    }

    #[test]
    fn validate_password_strength_accepts_normal_passwords() {
        assert!(validate_password_strength("mypassword").is_ok());
        assert!(validate_password_strength("SecurePass123!").is_ok());
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
    fn build_change_password_sql_returns_tuple() {
        let (query, modify) = build_change_password_sql("8.0.36", "secret");
        assert!(query.contains("SELECT User, Host"));
        assert!(modify.contains("UPDATE mysql.user"));
        assert!(modify.contains("secret"));
    }

    #[test]
    fn build_password_reset_sql_escapes_special_chars() {
        let sql = build_password_reset_sql("8.0.36", "pass'with\"special");
        assert!(sql.contains("pass''with\\\"special"));
    }
}
