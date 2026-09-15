//! Python 工具的 IPC 命令定义

use crate::types::*;
use tauri::{AppHandle, Window};

use super::detector;
use super::env_manager;
use super::mirror_manager;
use super::package_manager;
use super::version_fetcher;

#[tauri::command]
pub async fn detect_python(app_handle: AppHandle) -> Result<Vec<PythonVersion>, String> {
    detector::detect_python_versions(app_handle)
        .await
        .map_err(|e| format!("Python 检测失败: {}", e))
}

#[tauri::command]
pub async fn detect_default_python(app_handle: AppHandle) -> Result<Option<PythonVersion>, String> {
    detector::detect_default_python(app_handle)
        .await
        .map_err(|e| format!("Python 默认版本检测失败: {}", e))
}

#[tauri::command]
pub async fn list_python_environments(app_handle: AppHandle) -> Result<Vec<PythonEnvironment>, String> {
    env_manager::list_python_environments(app_handle)
        .await
        .map_err(|e| format!("Python 环境列表获取失败: {}", e))
}

#[tauri::command]
pub async fn list_python_packages(
    app_handle: AppHandle,
    python_path: Option<String>,
) -> Result<Vec<PythonPackage>, String> {
    package_manager::list_installed_packages(app_handle, python_path).await
}

#[tauri::command]
pub async fn list_python_mirrors(app_handle: AppHandle, python_path: Option<String>) -> Result<Vec<PipMirror>, String> {
    mirror_manager::list_pip_mirrors(app_handle, python_path)
        .await
        .map_err(|e| format!("Python 镜像列表获取失败: {}", e))
}

#[tauri::command]
pub async fn switch_python_mirror(
    app_handle: AppHandle,
    mirror_name: String,
    mirror_url: String,
    python_path: Option<String>,
) -> Result<String, String> {
    mirror_manager::switch_pip_mirror(app_handle, mirror_name, mirror_url, python_path).await
}

#[tauri::command]
pub async fn get_available_python_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailablePythonVersion>, String> {
    version_fetcher::get_available_python_versions(app_handle).await
}

#[tauri::command]
pub async fn download_python(
    app_handle: AppHandle,
    version: String,
    window: Window,
) -> Result<String, String> {
    version_fetcher::download_python_only(app_handle, version, window).await
}

#[tauri::command]
pub fn get_python_download_url(version: String) -> Result<String, String> {
    version_fetcher::validate_version_string(&version)?;
    Ok(version_fetcher::get_download_url(&version))
}
