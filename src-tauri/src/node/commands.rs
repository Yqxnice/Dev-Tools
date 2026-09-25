//! Node.js 工具的 IPC 命令定义

use crate::types::*;
use tauri::{AppHandle, Window};

use super::detector;
use super::mirror_manager;
use super::package_manager;
use super::version_fetcher;

#[tauri::command]
pub async fn detect_node(app_handle: AppHandle) -> Result<Vec<NodeVersion>, String> {
    detector::detect_node_versions(app_handle)
        .await
        .map_err(|e| format!("Node 检测失败: {}", e))
}

#[tauri::command]
pub async fn detect_default_node(app_handle: AppHandle) -> Result<Option<NodeVersion>, String> {
    detector::detect_default_node(app_handle)
        .await
        .map_err(|e| format!("Node 默认检测失败: {}", e))
}

#[tauri::command]
pub async fn list_node_packages(
    app_handle: AppHandle,
    node_path: Option<String>,
) -> Result<Vec<NodePackage>, String> {
    package_manager::list_installed_packages(app_handle, node_path).await
}

#[tauri::command]
pub async fn list_node_mirrors(app_handle: AppHandle) -> Result<Vec<NpmMirror>, String> {
    mirror_manager::list_npm_mirrors(app_handle)
        .await
        .map_err(|e| format!("Node 镜像列表获取失败: {}", e))
}

#[tauri::command]
pub async fn switch_node_mirror(
    app_handle: AppHandle,
    mirror_name: String,
    mirror_url: String,
) -> Result<String, String> {
    mirror_manager::switch_npm_mirror(app_handle, mirror_name, mirror_url).await
}

#[tauri::command]
pub async fn get_available_node_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailableNodeVersion>, String> {
    version_fetcher::get_available_node_versions(app_handle).await
}

/// 获取指定版本的下载链接（.msi 安装包）
#[tauri::command]
pub fn get_node_download_url(version: String) -> Result<String, String> {
    version_fetcher::validate_version_string(&version)?;
    Ok(version_fetcher::get_download_url(&version))
}

/// 下载指定版本的 Node.js 安装包
#[tauri::command]
pub async fn download_node(
    app_handle: AppHandle,
    version: String,
    window: Window,
) -> Result<String, String> {
    version_fetcher::download_node_only(app_handle, version, window).await
}
