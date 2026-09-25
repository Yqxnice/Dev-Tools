//! MySQL 工具的 IPC 命令定义
//!
//! 命令函数集中在此模块，由 MySqlPlugin::invoke_handler() 通过 generate_handler! 注册，
//! 避免在 lib.rs 中硬编码命令清单。

use crate::access;
use crate::process_manager;
use crate::types::*;
use tauri::{AppHandle, Window};

use super::cleaner;
use super::detector;
use super::fetcher;
use super::password_reset;
use super::uninstaller;

#[tauri::command]
pub async fn detect_mysql(app_handle: AppHandle) -> Result<MySQLInfo, String> {
    detector::detect_all_mysql(Some(&app_handle)).await
}

#[tauri::command]
pub async fn uninstall_mysql(
    app_handle: AppHandle,
    services: Option<Vec<String>>,
    instances: Option<Vec<MySQLInstance>>,
) -> Result<(), String> {
    access::require_admin()?;
    let services = services.unwrap_or_default();
    if services.is_empty() {
        return Err("请先选择要卸载的实例服务（未选择时操作已取消，避免误卸全部）".to_string());
    }
    for s in &services {
        process_manager::validate_service_name(s)?;
    }
    uninstaller::uninstall_selected_mysql(app_handle, services, instances.unwrap_or_default()).await
}

#[tauri::command]
pub async fn scan_mysql_residuals(
    app_handle: AppHandle,
    selected_instance: MySQLInstance,
) -> Result<CleanScanResult, String> {
    Ok(cleaner::scan_mysql_residuals(app_handle, selected_instance).await)
}

#[tauri::command]
pub async fn clean_mysql_residuals(
    app_handle: AppHandle,
    selected_instance: MySQLInstance,
    options: Option<CleanOptions>,
) -> Result<CleanResult, String> {
    access::require_admin()?;
    Ok(cleaner::clean_mysql_residuals(app_handle, selected_instance, options.unwrap_or_default()).await)
}

#[tauri::command]
pub async fn reset_mysql_password(
    app_handle: AppHandle,
    new_password: String,
    selected_instance: Option<MySQLInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    access::require_admin()?;
    password_reset::reset_mysql_password(app_handle, new_password, selected_instance, override_port).await
}

#[tauri::command]
pub async fn change_mysql_password(
    app_handle: AppHandle,
    old_password: String,
    new_password: String,
    selected_instance: Option<MySQLInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    access::require_admin()?;
    password_reset::change_mysql_password(app_handle, old_password, new_password, selected_instance, override_port).await
}

#[tauri::command]
pub async fn start_mysql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    process_manager::validate_service_name(&service_name)?;
    access::require_admin()?;
    detector::start_mysql_service(app_handle, service_name).await
}

#[tauri::command]
pub async fn stop_mysql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    process_manager::validate_service_name(&service_name)?;
    access::require_admin()?;
    detector::stop_mysql_service(app_handle, service_name).await
}

#[tauri::command]
pub async fn get_available_mysql_versions(app_handle: AppHandle) -> Result<Vec<MySQLVersionInfo>, String> {
    fetcher::get_available_mysql_versions_async(app_handle).await
}

#[tauri::command]
pub async fn download_mysql(
    app_handle: AppHandle,
    version: String,
    package_type: String,
    window: Window,
) -> Result<String, String> {
    fetcher::download_mysql(app_handle, version, package_type, window).await
}
