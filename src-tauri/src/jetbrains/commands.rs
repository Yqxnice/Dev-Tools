//! JetBrains 工具的 IPC 命令定义

use crate::access;
use crate::types::*;
use tauri::{AppHandle, Window};

use super::cleaner;
use super::detector;
use super::fetcher;
use super::uninstaller;

#[tauri::command]
pub async fn detect_jetbrains(app_handle: AppHandle) -> Result<Vec<JetBrainsInstallation>, String> {
    detector::detect_jetbrains(app_handle)
        .await
        .map_err(|e| format!("JetBrains 检测失败: {}", e))
}

#[tauri::command]
pub async fn get_available_jetbrains_versions(
    app_handle: AppHandle,
) -> Result<Vec<JetBrainsVersionInfo>, String> {
    fetcher::get_available_jetbrains_versions(app_handle).await
}

#[tauri::command]
pub async fn get_jetbrains_product_versions(
    app_handle: AppHandle,
    product_code: String,
) -> Result<Vec<JetBrainsVersionDetail>, String> {
    fetcher::get_jetbrains_product_versions(app_handle, product_code).await
}

#[tauri::command]
pub async fn download_jetbrains(
    app_handle: AppHandle,
    product_code: String,
    version: String,
    package_type: String,
    window: Window,
) -> Result<String, String> {
    fetcher::download_jetbrains(app_handle, product_code, version, package_type, window).await
}

#[tauri::command]
pub async fn uninstall_jetbrains(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> Result<(), String> {
    access::require_admin()?;
    uninstaller::uninstall_jetbrains(app_handle, installation).await
}

#[tauri::command]
pub async fn scan_jetbrains_residuals(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> Result<JetBrainsResidueScanResult, String> {
    Ok(cleaner::scan_jetbrains_residuals(app_handle, installation).await)
}

#[tauri::command]
pub async fn clean_jetbrains_residuals(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> Result<CleanResult, String> {
    access::require_admin()?;
    Ok(cleaner::clean_jetbrains_residuals(app_handle, installation).await)
}
