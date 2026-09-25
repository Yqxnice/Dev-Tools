use crate::types::*;
use tauri::{AppHandle, Window};

use super::detector;
use super::mirror_manager;
use super::version_fetcher;

#[tauri::command]
pub async fn detect_java(app_handle: AppHandle) -> Result<Vec<JavaVersion>, String> {
    detector::detect_java_versions(app_handle)
        .await
        .map_err(|e| format!("Java 检测失败: {}", e))
}

#[tauri::command]
pub async fn detect_default_java(app_handle: AppHandle) -> Result<Option<JavaVersion>, String> {
    detector::detect_default_java(app_handle)
        .await
        .map_err(|e| format!("Java 默认检测失败: {}", e))
}

#[tauri::command]
pub async fn get_available_java_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailableJavaVersion>, String> {
    version_fetcher::get_available_java_versions(app_handle).await
}

#[tauri::command]
pub fn get_java_download_url(version: String, package_type: Option<String>) -> Result<String, String> {
    let v = version_fetcher::validate_feature_version(&version)?;
    let pt = package_type.unwrap_or_else(|| version_fetcher::PACKAGE_TYPE_INSTALLER.to_string());
    Ok(version_fetcher::get_download_url(v, &pt))
}

#[tauri::command]
pub async fn list_java_mirrors(app_handle: AppHandle) -> Result<Vec<MavenMirror>, String> {
    mirror_manager::list_maven_mirrors(app_handle)
        .await
        .map_err(|e| format!("Java 镜像列表获取失败: {}", e))
}

#[tauri::command]
pub async fn switch_java_mirror(
    app_handle: AppHandle,
    mirror_name: String,
    mirror_url: String,
) -> Result<String, String> {
    mirror_manager::switch_maven_mirror(app_handle, mirror_name, mirror_url).await
}

/// 下载指定 feature version 的 Java JDK 安装包/压缩包
#[tauri::command]
pub async fn download_java(
    app_handle: AppHandle,
    version: String,
    package_type: Option<String>,
    window: Window,
) -> Result<String, String> {
    let v = version_fetcher::validate_feature_version(&version)?;
    let pt = package_type.unwrap_or_else(|| version_fetcher::PACKAGE_TYPE_INSTALLER.to_string());
    version_fetcher::download_java_only(app_handle, v, &pt, window).await
}
