//! PostgreSQL 工具的 IPC 命令定义

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
pub async fn detect_postgresql(app_handle: AppHandle) -> Result<PostgresqlInfo, String> {
    detector::detect_postgresql(Some(&app_handle)).await
}

#[tauri::command]
pub fn get_available_postgresql_versions() -> Result<Vec<PostgresqlVersionInfo>, String> {
    fetcher::get_available_postgresql_versions()
}

#[tauri::command]
pub async fn download_postgresql(
    app_handle: AppHandle,
    version: String,
    window: Window,
) -> Result<String, String> {
    fetcher::download_postgresql(app_handle, version, window).await
}

#[tauri::command]
pub async fn uninstall_postgresql(
    app_handle: AppHandle,
    services: Option<Vec<String>>,
    instances: Option<Vec<PostgresqlInstance>>,
) -> Result<(), String> {
    access::require_admin()?;
    let services = services.unwrap_or_default();
    if services.is_empty() {
        return Err("请先选择要卸载的实例服务（未选择时操作已取消，避免误卸全部）".to_string());
    }
    for s in &services {
        process_manager::validate_service_name(s)?;
    }
    uninstaller::uninstall_selected_postgresql(app_handle, services, instances.unwrap_or_default()).await
}

#[tauri::command]
pub async fn scan_postgresql_residuals(
    app_handle: AppHandle,
    selected_instance: PostgresqlInstance,
) -> Result<CleanScanResult, String> {
    Ok(cleaner::scan_postgresql_residuals(app_handle, selected_instance).await)
}

#[tauri::command]
pub async fn clean_postgresql_residuals(
    app_handle: AppHandle,
    selected_instance: PostgresqlInstance,
    options: Option<CleanOptions>,
) -> Result<CleanResult, String> {
    access::require_admin()?;
    Ok(cleaner::clean_postgresql_residuals(app_handle, selected_instance, options.unwrap_or_default()).await)
}

#[tauri::command]
pub async fn reset_postgresql_password(
    app_handle: AppHandle,
    new_password: String,
    selected_instance: Option<PostgresqlInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    access::require_admin()?;
    password_reset::reset_postgresql_password(app_handle, new_password, selected_instance, override_port).await
}

#[tauri::command]
pub async fn change_postgresql_password(
    app_handle: AppHandle,
    old_password: String,
    new_password: String,
    selected_instance: Option<PostgresqlInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    access::require_admin()?;
    password_reset::change_postgresql_password(app_handle, old_password, new_password, selected_instance, override_port).await
}

#[tauri::command]
pub async fn start_postgresql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    process_manager::validate_service_name(&service_name)?;
    access::require_admin()?;
    detector::start_postgresql_service(app_handle, service_name).await
}

#[tauri::command]
pub async fn stop_postgresql_service(app_handle: AppHandle, service_name: String) -> Result<(), String> {
    process_manager::validate_service_name(&service_name)?;
    access::require_admin()?;
    detector::stop_postgresql_service(app_handle, service_name).await
}
