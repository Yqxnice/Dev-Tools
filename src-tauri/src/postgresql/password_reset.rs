use super::super::{logger, process_manager, types};
use super::detector;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// 转义 PostgreSQL 密码字符串中的单引号（SQL 标准双单引号）
pub fn escape_postgresql_password(password: &str) -> String {
    password.replace('\'', "''")
}

/// 验证密码强度（复用 process_manager 的通用验证）
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    process_manager::validate_password_strength(password).map_err(|e| e.to_user_message())
}

/// 轮询等待服务达到目标状态
async fn wait_for_service_state(
    app_handle: &AppHandle,
    service_name: &str,
    running: bool,
    timeout_secs: u64,
) -> bool {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let status = crate::service_manager::check_service_status(service_name).await;
        let reached = if running { status == "启动" } else { status != "启动" };
        if reached { return true; }
        if tokio::time::Instant::now() >= deadline {
            logger::warn(app_handle, &format!("等待服务 {} 状态超时（当前: {}）", service_name, status));
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// 轮询等待端口可连接
async fn wait_for_port_ready(app_handle: &AppHandle, port: u16, timeout_secs: u64) -> bool {
    use tokio::net::TcpStream;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let connect = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            TcpStream::connect(("127.0.0.1", port)),
        ).await;
        if matches!(connect, Ok(Ok(_))) { return true; }
        if tokio::time::Instant::now() >= deadline {
            logger::warn(app_handle, &format!("等待端口 {} 就绪超时", port));
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// 查找 pg_hba.conf 文件（与 postgresql.conf 同目录）
async fn find_pg_hba_conf(bin_path: &str, data_dir: Option<&str>) -> Option<PathBuf> {
    // 1. 优先从已知的 data_dir 查找
    if let Some(dd) = data_dir {
        let pg_hba = Path::new(dd).join("pg_hba.conf");
        if pg_hba.exists() {
            return Some(pg_hba);
        }
    }
    // 2. 从 bin 目录推导 data 目录
    if let Some(config) = detector::get_postgresql_config_file(bin_path).await {
        if let Some(parent) = config.parent() {
            let pg_hba = parent.join("pg_hba.conf");
            if pg_hba.exists() {
                return Some(pg_hba);
            }
        }
    }
    None
}

/// Guard：确保 pg_hba.conf 在流程结束时恢复原状（即使 panic 也执行）
struct PgHbaGuard {
    backup: Option<PathBuf>,
    target: PathBuf,
}

impl PgHbaGuard {
    async fn restore(&self, app_handle: &AppHandle) {
        if let Some(backup) = &self.backup {
            let _ = tokio::fs::copy(backup, &self.target).await;
            let _ = tokio::fs::remove_file(backup).await;
            logger::info(app_handle, &format!("已恢复 pg_hba.conf: {}", self.target.display()));
        }
    }
}

impl Drop for PgHbaGuard {
    fn drop(&mut self) {
        if let Some(backup) = self.backup.take() {
            let target = self.target.clone();
            let _ = std::fs::copy(&backup, &target);
            let _ = std::fs::remove_file(&backup);
        }
    }
}

/// 重置流程失败后的兜底恢复：恢复 pg_hba.conf 并重启原服务
async fn restore_service_after_failure(
    app_handle: &AppHandle,
    service_name: &str,
    guard: &PgHbaGuard,
) {
    logger::warn(app_handle, "重置流程异常中断，开始自动恢复：恢复 pg_hba.conf 并重启原服务...");
    guard.restore(app_handle).await;
    let _ = detector::start_postgresql_service(app_handle.clone(), service_name.to_string()).await;
    let restarted = wait_for_service_state(app_handle, service_name, true, 30).await;
    if !restarted {
        logger::error(app_handle, &format!(
            "自动恢复服务失败，请手动在管理员终端执行 `net start {}` 启动服务", service_name));
    } else {
        logger::info(app_handle, "自动恢复完成：pg_hba.conf 已恢复，原服务已重新启动");
    }
}

fn resolve_port(instance: &types::PostgresqlInstance, override_port: Option<u16>) -> Option<u16> {
    override_port.or(instance.port)
}

/// 重置 PostgreSQL postgres 用户密码
///
/// 实现方式：pg_hba trust 法
/// 1. 停止服务
/// 2. 备份 pg_hba.conf，写入临时 trust 配置
/// 3. 启动服务
/// 4. psql -U postgres -h 127.0.0.1 -c "ALTER USER postgres PASSWORD 'newpass';"
/// 5. 停止服务，恢复 pg_hba.conf
/// 6. 启动服务
/// 7. 用新密码测试连接
pub async fn reset_postgresql_password(
    app_handle: AppHandle,
    new_password: String,
    selected_instance: Option<types::PostgresqlInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    validate_password_strength(&new_password)?;

    logger::info(&app_handle, "========================================");
    logger::info(&app_handle, "开始自动重置 PostgreSQL 密码");
    logger::info(&app_handle, "========================================");

    let pg_info = detector::detect_postgresql(Some(&app_handle)).await?;
    logger::info(&app_handle, &format!("检测到 {} 个 PostgreSQL 实例", pg_info.instances.len()));

    let instance = crate::detector_base::select_valid_instance(
        selected_instance.as_ref(),
        &pg_info.instances,
        "PostgreSQL",
    )?;
    logger::info(&app_handle, &format!(
        "使用实例: 版本 {}, 服务 {:?}, 路径 {}",
        instance.version, instance.service_name, instance.path
    ));

    let service_name = match &instance.service_name {
        Some(name) => name.clone(),
        None => {
            logger::error(&app_handle, "所选实例没有服务名，无法重置密码");
            return Err("所选实例没有服务名，无法重置密码".to_string());
        }
    };
    let bin_path = &instance.path;
    let port = resolve_port(instance, override_port);
    if let Some(p) = port {
        logger::info(&app_handle, &format!("使用端口: {} ({})", p, if override_port.is_some() { "手动指定" } else { "自动检测" }));
    }

    let psql_path = PathBuf::from(bin_path).join("psql.exe");
    logger::info(&app_handle, &format!("psql 可执行文件: {:?}", psql_path));
    if !psql_path.exists() {
        logger::error(&app_handle, &format!("未找到 psql.exe: {:?}", psql_path));
        return Err(format!("未找到 psql.exe: {:?}", psql_path));
    }

    // 1. 停止 PostgreSQL 服务
    logger::info(&app_handle, &format!("正在停止 PostgreSQL 服务: {}", service_name));
    match process_manager::execute_command("net", &["stop", &service_name]).await {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::warn(&app_handle, &format!("停止服务警告: {}", output.stderr.trim()));
            }
        }
        Err(e) => {
            logger::warn(&app_handle, &format!("停止服务失败: {}", e));
        }
    }
    wait_for_service_state(&app_handle, &service_name, false, 30).await;

    // 终止残留进程
    if !bin_path.is_empty() {
        logger::info(&app_handle, &format!("终止该实例的 PostgreSQL 进程（目录: {}）...", bin_path));
        match detector::kill_postgresql_processes_in_dir(bin_path).await {
            Ok(n) => logger::info(&app_handle, &format!("已终止 {} 个实例进程", n)),
            Err(e) => logger::warn(&app_handle, &format!("终止进程失败: {}", e)),
        }
        detector::wait_postgresql_processes_gone(&app_handle, bin_path).await;
    }

    // 2. 查找并备份 pg_hba.conf
    let pg_hba_path = find_pg_hba_conf(bin_path, instance.data_dir.as_deref()).await;
    let pg_hba_path = match pg_hba_path {
        Some(p) => {
            logger::info(&app_handle, &format!("找到 pg_hba.conf: {}", p.display()));
            p
        }
        None => {
            logger::error(&app_handle, "未找到 pg_hba.conf，无法重置密码");
            let _ = detector::start_postgresql_service(app_handle.clone(), service_name.clone()).await;
            return Err("未找到 pg_hba.conf，无法通过 trust 法重置密码".to_string());
        }
    };

    // 备份 pg_hba.conf
    let backup_path = pg_hba_path.with_extension("conf.bak.devtools");
    tokio::fs::copy(&pg_hba_path, &backup_path).await
        .map_err(|e| {
            let _ = detector::start_postgresql_service(app_handle.clone(), service_name.clone());
            format!("备份 pg_hba.conf 失败: {}", e)
        })?;
    logger::info(&app_handle, &format!("已备份 pg_hba.conf 到: {}", backup_path.display()));

    let guard = PgHbaGuard {
        backup: Some(backup_path.clone()),
        target: pg_hba_path.clone(),
    };

    // 写入临时 trust 配置
    let trust_config = "# TYPE  DATABASE  USER  ADDRESS  METHOD\n\
        host    all       all   127.0.0.1/32  trust\n\
        host    all       all   ::1/128      trust\n\
        local   all       all                  trust\n";
    tokio::fs::write(&pg_hba_path, trust_config).await
        .map_err(|e| {
            let _ = std::fs::copy(&backup_path, &pg_hba_path);
            let _ = std::fs::remove_file(&backup_path);
            let _ = detector::start_postgresql_service(app_handle.clone(), service_name.clone());
            format!("写入临时 pg_hba.conf 失败: {}", e)
        })?;
    logger::info(&app_handle, "已写入临时 trust 认证配置");

    // 3. 启动服务（加载 trust 配置）
    logger::info(&app_handle, "正在以 trust 认证模式启动 PostgreSQL 服务...");
    match process_manager::execute_command("net", &["start", &service_name]).await {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::error(&app_handle, &format!("启动服务失败: {}", output.stderr));
                restore_service_after_failure(&app_handle, &service_name, &guard).await;
                return Err(format!("启动服务失败: {}", output.stderr));
            }
            logger::info(&app_handle, "PostgreSQL 服务已启动（trust 模式）");
        }
        Err(e) => {
            logger::error(&app_handle, &format!("启动服务异常: {}", e));
            restore_service_after_failure(&app_handle, &service_name, &guard).await;
            return Err(format!("启动服务异常: {}", e));
        }
    }

    // 等待服务就绪
    let service_ready = wait_for_service_state(&app_handle, &service_name, true, 40).await;
    let port_to_check = port.unwrap_or(5432);
    let port_ready = wait_for_port_ready(&app_handle, port_to_check, 40).await;
    if !service_ready || !port_ready {
        logger::warn(&app_handle, "服务或端口未就绪，但仍将尝试执行密码修改");
    }

    // 4. 执行 ALTER USER 修改密码
    let escaped = escape_postgresql_password(&new_password);
    let alter_sql = format!("ALTER USER postgres PASSWORD '{}';", escaped);
    let masked_sql = alter_sql.replace(&escaped, "***");
    logger::info(&app_handle, &format!("执行 SQL (密码已掩码): {}", masked_sql));

    let psql_str = psql_path.to_str().unwrap_or("");
    let args: Vec<String> = vec![
        "-U".to_string(), "postgres".to_string(),
        "-h".to_string(), "127.0.0.1".to_string(),
        "-p".to_string(), port_to_check.to_string(),
        "-c".to_string(), alter_sql,
    ];
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let result = process_manager::execute_command(psql_str, &args_ref).await;
    let mut sql_success = false;
    match result {
        Ok(output) => {
            logger::info(&app_handle, &format!("SQL 执行退出码: {}", output.exit_code));
            if output.exit_code == 0 {
                sql_success = true;
                logger::info(&app_handle, "密码修改 SQL 执行成功！");
            } else {
                logger::error(&app_handle, &format!("SQL 执行失败: {}", output.stderr));
            }
            if !output.stdout.is_empty() {
                logger::info(&app_handle, &format!("SQL 输出: {}", output.stdout));
            }
        }
        Err(e) => {
            logger::error(&app_handle, &format!("SQL 执行异常: {}", e));
        }
    }

    // 5. 停止服务
    logger::info(&app_handle, "正在停止 trust 模式的 PostgreSQL 服务...");
    match process_manager::execute_command("net", &["stop", &service_name]).await {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::warn(&app_handle, &format!("停止服务警告: {}", output.stderr.trim()));
            }
        }
        Err(e) => logger::warn(&app_handle, &format!("停止服务失败: {}", e)),
    }
    wait_for_service_state(&app_handle, &service_name, false, 30).await;

    // 终止残留进程（确保配置文件可写）
    if !bin_path.is_empty() {
        let _ = detector::kill_postgresql_processes_in_dir(bin_path).await;
        detector::wait_postgresql_processes_gone(&app_handle, bin_path).await;
    }

    // 6. 恢复 pg_hba.conf
    guard.restore(&app_handle).await;

    // 7. 启动服务（恢复正常认证模式）
    logger::info(&app_handle, "正在以正常认证模式重启 PostgreSQL 服务...");
    match process_manager::execute_command("net", &["start", &service_name]).await {
        Ok(output) => {
            if output.exit_code != 0 {
                logger::warn(&app_handle, &format!("启动服务警告: {}", output.stderr));
            }
        }
        Err(e) => logger::warn(&app_handle, &format!("启动服务异常: {}", e)),
    }
    wait_for_service_state(&app_handle, &service_name, true, 40).await;
    wait_for_port_ready(&app_handle, port_to_check, 40).await;

    // 8. 用新密码测试连接
    logger::info(&app_handle, "开始用新密码测试连接...");
    let port_str = port_to_check.to_string();

    let mut connection_ok = false;
    for retry in 1..=5u32 {
        logger::info(&app_handle, &format!("连接测试尝试 {}/5...", retry));
        let mut cmd = tokio::process::Command::new(psql_str);
        cmd.args(["-U", "postgres", "-h", "127.0.0.1", "-p", &port_str, "-c", "SELECT 1;"]);
        cmd.env("PGPASSWORD", &new_password);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000);
        }
        match cmd.output().await {
            Ok(output) => {
                if output.status.code() == Some(0) {
                    logger::info(&app_handle, "PostgreSQL 连接测试成功！");
                    connection_ok = true;
                    break;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    logger::warn(&app_handle, &format!("连接测试失败: {}", stderr.trim()));
                }
            }
            Err(e) => logger::warn(&app_handle, &format!("连接测试异常: {}", e)),
        }
        if retry < 5 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    if sql_success && connection_ok {
        logger::info(&app_handle, "========================================");
        logger::info(&app_handle, "密码重置成功！连接测试通过！");
        logger::info(&app_handle, "========================================");
        Ok("密码重置成功，连接测试通过！".to_string())
    } else if sql_success {
        logger::warn(&app_handle, "========================================");
        logger::warn(&app_handle, "连接测试失败，但密码可能已成功设置！");
        logger::warn(&app_handle, "========================================");
        Ok("密码可能已成功设置！虽然连接测试失败，但 SQL 命令执行成功。请尝试手动连接。".to_string())
    } else {
        logger::error(&app_handle, "========================================");
        logger::error(&app_handle, "密码重置失败！SQL 执行未通过");
        logger::error(&app_handle, "========================================");
        Err("密码重置失败，SQL 执行未通过，请查看日志获取详细信息".to_string())
    }
}

/// 修改 PostgreSQL postgres 用户密码（需要原密码）
pub async fn change_postgresql_password(
    app_handle: AppHandle,
    old_password: String,
    new_password: String,
    selected_instance: Option<types::PostgresqlInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    validate_password_strength(&new_password)?;

    let pg_info = detector::detect_postgresql(Some(&app_handle)).await?;

    let instance = if let Some(sel_inst) = &selected_instance {
        sel_inst
    } else {
        pg_info
            .instances
            .iter()
            .find(|inst| !inst.path.is_empty())
            .ok_or_else(|| "未找到 PostgreSQL 安装路径".to_string())?
    };

    let psql_path = PathBuf::from(&instance.path).join("psql.exe");
    let port = resolve_port(instance, override_port);
    if !psql_path.exists() {
        return Err("psql.exe 未找到".to_string());
    }

    let port_to_use = port.unwrap_or(5432);
    logger::info(&app_handle, "========== 开始修改密码 ==========");

    let psql_str = psql_path.to_str().ok_or("psql 路径无效")?;
    let port_str = port_to_use.to_string();

    // 先用旧密码验证连接
    logger::info(&app_handle, "正在验证旧密码...");
    let mut verify_cmd = tokio::process::Command::new(psql_str);
    verify_cmd.args(["-U", "postgres", "-h", "127.0.0.1", "-p", &port_str, "-c", "SELECT 1;"]);
    verify_cmd.env("PGPASSWORD", &old_password);
    verify_cmd.stdout(std::process::Stdio::piped());
    verify_cmd.stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        verify_cmd.creation_flags(0x08000000);
    }

    match verify_cmd.output().await {
        Ok(output) => {
            if output.status.code() != Some(0) {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("authentication failed") || stderr.contains("password") {
                    logger::error(&app_handle, "旧密码不正确！");
                    return Err("旧密码不正确！请确认后重试，或使用密码重置功能。".to_string());
                }
            } else {
                logger::info(&app_handle, "旧密码验证成功！");
            }
        }
        Err(e) => logger::warn(&app_handle, &format!("验证过程异常: {}", e)),
    }

    // 执行 ALTER USER
    let escaped = escape_postgresql_password(&new_password);
    let alter_sql = format!("ALTER USER postgres PASSWORD '{}';", escaped);
    let masked_sql = alter_sql.replace(&escaped, "***");
    logger::info(&app_handle, &format!("执行 SQL (密码已掩码): {}", masked_sql));

    let mut alter_cmd = tokio::process::Command::new(psql_str);
    alter_cmd.args(["-U", "postgres", "-h", "127.0.0.1", "-p", &port_str, "-c", &alter_sql]);
    alter_cmd.env("PGPASSWORD", &old_password);
    alter_cmd.stdout(std::process::Stdio::piped());
    alter_cmd.stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        alter_cmd.creation_flags(0x08000000);
    }

    match alter_cmd.output().await {
        Ok(output) => {
            if output.status.code() == Some(0) {
                logger::info(&app_handle, "密码修改命令执行成功！");
                // 用新密码测试
                let mut test_cmd = tokio::process::Command::new(psql_str);
                test_cmd.args(["-U", "postgres", "-h", "127.0.0.1", "-p", &port_str, "-c", "SELECT 1;"]);
                test_cmd.env("PGPASSWORD", &new_password);
                test_cmd.stdout(std::process::Stdio::piped());
                test_cmd.stderr(std::process::Stdio::piped());
                #[cfg(target_os = "windows")]
                {
                    test_cmd.creation_flags(0x08000000);
                }
                match test_cmd.output().await {
                    Ok(out) if out.status.code() == Some(0) => {
                        logger::info(&app_handle, "========== 密码修改成功！连接测试通过！ ==========");
                        Ok("密码修改成功，连接测试通过！".to_string())
                    }
                    _ => {
                        logger::warn(&app_handle, "连接测试失败，但密码可能已修改成功");
                        Ok("密码可能已成功修改！虽然连接测试失败，但 SQL 命令执行成功。请尝试手动连接。".to_string())
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                logger::error(&app_handle, &format!("密码修改失败: {}", stderr));
                Err(format!("密码修改失败: {}", stderr))
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
    fn escape_password_doubles_single_quotes() {
        assert_eq!(escape_postgresql_password("normal"), "normal");
        assert_eq!(escape_postgresql_password("it's"), "it''s");
        assert_eq!(escape_postgresql_password("a'b'c"), "a''b''c");
    }

    #[test]
    fn validate_password_rejects_short() {
        assert!(validate_password_strength("123").is_err());
        assert!(validate_password_strength("12345").is_err());
    }

    #[test]
    fn validate_password_accepts_normal() {
        assert!(validate_password_strength("123456").is_ok());
        assert!(validate_password_strength("Password1").is_ok());
        assert!(validate_password_strength("SecurePass123!").is_ok());
    }
}
