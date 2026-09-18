//! 通用 Windows 服务管理：MySQL/PostgreSQL 等数据库工具共享的服务操作
//!
//! 通过 windows crate 的 ServiceManager API 直接调用，替代 sc.exe 命令字符串解析。
//! 所有同步 Win32 API 调用包装在 `tokio::task::spawn_blocking` 中，避免阻塞 runtime。

use crate::{detector_base, error::{AppError, AppResult}, logger};
use tauri::AppHandle;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, DeleteService, EnumServicesStatusExW, OpenSCManagerW,
    OpenServiceW, QueryServiceConfigW, QueryServiceStatus, StartServiceW, ENUM_SERVICE_STATE,
    ENUM_SERVICE_STATUS_PROCESSW, QUERY_SERVICE_CONFIGW, SC_ENUM_PROCESS_INFO, SERVICE_CONTROL_STOP,
    SERVICE_STATUS, SERVICE_WIN32,
};

/// 标准服务访问权限掩码（windows crate 中对应常量为 u32）
const SC_MANAGER_CONNECT: u32 = 1;
const SC_MANAGER_ENUMERATE_SERVICE: u32 = 4;
const SERVICE_QUERY_CONFIG: u32 = 1;
const SERVICE_QUERY_STATUS: u32 = 4;
const SERVICE_START: u32 = 16;
const SERVICE_STOP: u32 = 32;
/// DELETE 访问权限（标准权限位 0x00010000），用于 DeleteService
const DELETE_ACCESS: u32 = 0x00010000;

/// 服务状态枚举值（SERVICE_STATUS_CURRENT_STATE.0）
const SERVICE_RUNNING_STATE: u32 = 4;
/// SERVICE_STATE_ALL
const SERVICE_STATE_ALL: u32 = 3;

/// 将 &str 转换为以 null 结尾的 UTF-16 宽字符序列（crate 内共享）
pub(crate) fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 将 PWSTR（指向 UTF-16 字符串的裸指针包装）转换为 Rust String
unsafe fn pwstr_to_string(pwsr: PWSTR) -> String {
    if pwsr.is_null() {
        return String::new();
    }
    let ptr = pwsr.as_ptr();
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

/// 从 Windows 服务的 lpBinaryPathName 命令行中提取纯可执行文件路径。
///
/// lpBinaryPathName 可能是以下形式：
/// - `"C:\path\to\exe.exe" --args` （带引号，路径含空格）
/// - `C:\path\to\exe.exe --args`  （无引号，路径无空格）
/// - `"C:\path to\exe.exe"`       （仅有路径）
///
/// 提取策略：
/// 1. 若以 `"` 开头，取首对引号之间的内容
/// 2. 否则按空格分割，从最长前缀开始检查文件是否存在，返回首个匹配
/// 3. 兜底返回去掉首个空格后参数部分之前的内容
fn extract_exe_path(command_line: &str) -> String {
    let trimmed = command_line.trim();
    // 形式 1：以引号开头
    if trimmed.starts_with('"') {
        if let Some(end) = trimmed[1..].find('"') {
            return trimmed[1..1 + end].to_string();
        }
    }
    // 形式 2：无引号，尝试按空格逐步缩短前缀检查文件是否存在
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.is_empty() {
        return trimmed.to_string();
    }
    // 先尝试第一段（最常见：路径无空格）
    if std::path::Path::new(parts[0]).exists() {
        return parts[0].to_string();
    }
    // 路径含空格但无引号：逐步拼接前两段、前三段...检查是否存在
    let mut acc = String::new();
    for (i, part) in parts.iter().enumerate() {
        if !acc.is_empty() {
            acc.push(' ');
        }
        acc.push_str(part);
        if std::path::Path::new(&acc).exists() {
            return acc;
        }
        // 防止过度拼接（限制在前 5 段内）
        if i >= 4 {
            break;
        }
    }
    // 兜底：返回首个空格之前的部分
    parts[0].to_string()
}

/// 查询指定 Windows 服务是否存在
pub async fn check_service_exists(service_name: &str) -> bool {
    let name = service_name.to_string();
    tokio::task::spawn_blocking(move || -> bool {
        unsafe {
            let scm = match OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) {
                Ok(h) => h,
                Err(_) => return false,
            };
            let wide = to_wide(&name);
            let svc = match OpenServiceW(scm, PCWSTR(wide.as_ptr()), SERVICE_QUERY_STATUS) {
                Ok(h) => h,
                Err(_) => {
                    let _ = CloseServiceHandle(scm);
                    return false;
                }
            };
            let _ = CloseServiceHandle(svc);
            let _ = CloseServiceHandle(scm);
            true
        }
    })
    .await
    .unwrap_or(false)
}

/// 查询服务状态：返回 STATUS_RUNNING / STATUS_STOPPED / STATUS_NOT_INSTALLED
pub async fn check_service_status(service_name: &str) -> String {
    let name = service_name.to_string();
    tokio::task::spawn_blocking(move || -> String {
        unsafe {
            let scm = match OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) {
                Ok(h) => h,
                Err(_) => return detector_base::STATUS_NOT_INSTALLED.to_string(),
            };
            let wide = to_wide(&name);
            let svc = match OpenServiceW(scm, PCWSTR(wide.as_ptr()), SERVICE_QUERY_STATUS) {
                Ok(h) => h,
                Err(_) => {
                    let _ = CloseServiceHandle(scm);
                    return detector_base::STATUS_NOT_INSTALLED.to_string();
                }
            };
            let mut status = SERVICE_STATUS::default();
            let ok = QueryServiceStatus(svc, &mut status).is_ok();
            let _ = CloseServiceHandle(svc);
            let _ = CloseServiceHandle(scm);
            if !ok {
                return detector_base::STATUS_NOT_INSTALLED.to_string();
            }
            let current = status.dwCurrentState.0;
            if current == SERVICE_RUNNING_STATE {
                detector_base::STATUS_RUNNING.to_string()
            } else {
                // STOPPED / START_PENDING / STOP_PENDING 等统一归为停止
                detector_base::STATUS_STOPPED.to_string()
            }
        }
    })
    .await
    .unwrap_or_else(|_| detector_base::STATUS_NOT_INSTALLED.to_string())
}

/// 获取服务对应的可执行文件所在目录（bin 目录）
pub(crate) async fn get_service_binary_path(service_name: &str) -> Option<String> {
    let name = service_name.to_string();
    tokio::task::spawn_blocking(move || -> Option<String> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT).ok()?;
            let wide = to_wide(&name);
            let svc = OpenServiceW(scm, PCWSTR(wide.as_ptr()), SERVICE_QUERY_CONFIG).ok()?;
            // 第一次调用：用 None 探测所需 buffer 大小
            let mut bytes_needed: u32 = 0;
            let _ = QueryServiceConfigW(svc, None, 0, &mut bytes_needed);
            if bytes_needed == 0 {
                let _ = CloseServiceHandle(svc);
                let _ = CloseServiceHandle(scm);
                return None;
            }
            // 第二次调用：分配足够 buffer 获取实际配置
            let mut buffer = vec![0u8; bytes_needed as usize];
            let config_ptr = buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;
            let mut needed: u32 = 0;
            let result = QueryServiceConfigW(svc, Some(config_ptr), bytes_needed, &mut needed);
            let _ = CloseServiceHandle(svc);
            let _ = CloseServiceHandle(scm);
            if result.is_err() {
                return None;
            }
            // config_ptr 指向 buffer 起始处的 QUERY_SERVICE_CONFIGW，
            // 其 lpBinaryPathName 指向 buffer 内的字符串数据
            let config = &*config_ptr;
            let bin_path = pwstr_to_string(config.lpBinaryPathName);
            if bin_path.is_empty() {
                return None;
            }
            // lpBinaryPathName 是完整命令行（可能含参数，如
            //   "C:\path\to\mysqld.exe" --defaults-file="..."
            // 需要先提取纯 exe 路径，再取 parent 目录
            let exe_path = extract_exe_path(&bin_path);
            let path = std::path::Path::new(&exe_path);
            path.parent().and_then(|p| p.to_str()).map(|s| s.to_string())
        }
    })
    .await
    .ok()
    .flatten()
}

/// 按关键词查找所有匹配的 Windows 服务名
pub async fn find_services_by_keywords(keywords: &[&str]) -> Vec<String> {
    let owned_keywords: Vec<String> = keywords.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || -> Vec<String> {
        unsafe {
            let scm = match OpenSCManagerW(
                PCWSTR::null(),
                PCWSTR::null(),
                SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE,
            ) {
                Ok(h) => h,
                Err(_) => return Vec::new(),
            };
            // 第一次调用：用 None 探测所需 buffer 大小
            let mut bytes_needed: u32 = 0;
            let mut services_returned: u32 = 0;
            let mut resume_handle: u32 = 0;
            let _ = EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                ENUM_SERVICE_STATE(SERVICE_STATE_ALL),
                None,
                &mut bytes_needed,
                &mut services_returned,
                Some(&mut resume_handle as *mut u32),
                PCWSTR::null(),
            );
            if bytes_needed == 0 {
                let _ = CloseServiceHandle(scm);
                return Vec::new();
            }
            // 第二次调用：分配足够 buffer 获取实际数据
            let mut buffer = vec![0u8; bytes_needed as usize];
            let mut needed: u32 = 0;
            let mut returned: u32 = 0;
            let mut resume: u32 = 0;
            let result = EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                ENUM_SERVICE_STATE(SERVICE_STATE_ALL),
                Some(buffer.as_mut_slice()),
                &mut needed,
                &mut returned,
                Some(&mut resume as *mut u32),
                PCWSTR::null(),
            );
            let _ = CloseServiceHandle(scm);
            if result.is_err() {
                return Vec::new();
            }
            // 解析 buffer 为 ENUM_SERVICE_STATUS_PROCESSW 数组
            let entries: &[ENUM_SERVICE_STATUS_PROCESSW] = std::slice::from_raw_parts(
                buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW,
                returned as usize,
            );
            let mut services = Vec::with_capacity(returned as usize);
            for entry in entries {
                let svc_name = pwstr_to_string(entry.lpServiceName);
                let lower = svc_name.to_lowercase();
                if owned_keywords.iter().any(|kw| lower.contains(kw)) {
                    services.push(svc_name);
                }
            }
            services
        }
    })
    .await
    .unwrap_or_default()
}

/// 通用启动服务（tool_name 仅用于日志）
pub async fn start_service(
    app_handle: &AppHandle,
    tool_name: &str,
    service_name: &str,
) -> AppResult<()> {
    logger::info(
        app_handle,
        &format!("正在启动 {} 服务: {}", tool_name, service_name),
    );
    let svc = service_name.to_string();
    let result = tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
                .map_err(|e| AppError::CommandExecution(format!("打开 SCM 失败: {}", e)))?;
            let wide = to_wide(&svc);
            let handle = OpenServiceW(scm, PCWSTR(wide.as_ptr()), SERVICE_START).map_err(|e| {
                let _ = CloseServiceHandle(scm);
                AppError::CommandExecution(format!("打开服务失败: {}", e))
            })?;
            let result = StartServiceW(handle, None);
            let _ = CloseServiceHandle(handle);
            let _ = CloseServiceHandle(scm);
            result.map_err(|e| AppError::CommandExecution(format!("启动服务失败: {}", e)))
        }
    })
    .await
    .map_err(|e| AppError::CommandExecution(format!("启动任务异常: {}", e)))?;
    match result {
        Ok(()) => {
            // StartServiceW 返回后服务应已进入 RUNNING，但大型数据库（MySQL 等）
            // 可能存在 SCM 状态传播延迟。轮询确认，防止紧随其后的 detect 读到
            // START_PENDING（被 check_service_status 归为"停止"）导致 UI 不更新。
            let svc_name = service_name.to_string();
            for _ in 0..10 {
                let status = check_service_status(&svc_name).await;
                if status == detector_base::STATUS_RUNNING {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            logger::info(
                app_handle,
                &format!("{} 服务 {} 启动成功", tool_name, service_name),
            );
            Ok(())
        }
        Err(error_msg) => {
            logger::error(
                app_handle,
                &format!("{} 服务 {} 启动失败: {}", tool_name, service_name, error_msg),
            );
            Err(error_msg)
        }
    }
}

/// 通用停止服务（tool_name 仅用于日志）
pub async fn stop_service(
    app_handle: &AppHandle,
    tool_name: &str,
    service_name: &str,
) -> AppResult<()> {
    logger::info(
        app_handle,
        &format!("正在停止 {} 服务: {}", tool_name, service_name),
    );
    let svc = service_name.to_string();
    let result = tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
                .map_err(|e| AppError::CommandExecution(format!("打开 SCM 失败: {}", e)))?;
            let wide = to_wide(&svc);
            let handle = OpenServiceW(scm, PCWSTR(wide.as_ptr()), SERVICE_STOP).map_err(|e| {
                let _ = CloseServiceHandle(scm);
                AppError::CommandExecution(format!("打开服务失败: {}", e))
            })?;
            let mut status = SERVICE_STATUS::default();
            let result = ControlService(handle, SERVICE_CONTROL_STOP, &mut status);
            let _ = CloseServiceHandle(handle);
            let _ = CloseServiceHandle(scm);
            result.map_err(|e| AppError::CommandExecution(format!("停止服务失败: {}", e)))
        }
    })
    .await
    .map_err(|e| AppError::CommandExecution(format!("停止任务异常: {}", e)))?;
    match result {
        Ok(()) => {
            logger::info(
                app_handle,
                &format!("{} 服务 {} 停止成功", tool_name, service_name),
            );
            Ok(())
        }
        Err(error_msg) => {
            logger::error(
                app_handle,
                &format!("{} 服务 {} 停止失败: {}", tool_name, service_name, error_msg),
            );
            Err(error_msg)
        }
    }
}

/// 标记指定服务为删除（卸载场景使用）
pub async fn delete_service(service_name: &str) -> AppResult<()> {
    let svc = service_name.to_string();
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        unsafe {
            let scm = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)
                .map_err(|e| AppError::CommandExecution(format!("打开 SCM 失败: {}", e)))?;
            let wide = to_wide(&svc);
            let handle = OpenServiceW(scm, PCWSTR(wide.as_ptr()), DELETE_ACCESS).map_err(|e| {
                let _ = CloseServiceHandle(scm);
                AppError::CommandExecution(format!("打开服务失败: {}", e))
            })?;
            let result = DeleteService(handle);
            let _ = CloseServiceHandle(handle);
            let _ = CloseServiceHandle(scm);
            result.map_err(|e| AppError::CommandExecution(format!("删除服务失败: {}", e)))
        }
    })
    .await
    .map_err(|e| AppError::CommandExecution(format!("删除任务异常: {}", e)))?
}
