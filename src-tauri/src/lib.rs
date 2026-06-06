#[cfg(not(target_os = "windows"))]
compile_error!("Dev Tools 仅支持 Windows 平台");

pub mod access;
pub mod types;
pub mod logger;
pub mod process_manager;
pub mod mysql;
pub mod python;

use types::*;
use check_elevation::is_elevated;

#[tauri::command]
fn is_running_as_admin() -> bool {
    is_elevated().unwrap_or(false)
}

#[tauri::command]
fn set_guest_mode(enabled: bool) {
    access::set_guest_mode(enabled);
}

#[tauri::command]
async fn detect_mysql(app_handle: tauri::AppHandle) -> MySQLInfo {
    mysql::detector::detect_all_mysql(Some(&app_handle)).await
}

#[tauri::command]
async fn uninstall_mysql(app_handle: tauri::AppHandle, services: Option<Vec<String>>) -> Result<(), String> {
    access::require_admin()?;
    mysql::uninstaller::uninstall_selected_mysql(app_handle, services.unwrap_or_default()).await
}

#[tauri::command]
async fn scan_mysql_residuals(
    app_handle: tauri::AppHandle,
    selected_instance: types::MySQLInstance,
) -> CleanScanResult {
    mysql::cleaner::scan_mysql_residuals(app_handle, selected_instance).await
}

#[tauri::command]
async fn clean_mysql_residuals(
    app_handle: tauri::AppHandle,
    selected_instance: types::MySQLInstance,
    options: Option<CleanOptions>,
) -> CleanResult {
    if let Err(e) = access::require_admin() {
        return CleanResult {
            success: false,
            message: e,
            cleaned_items: Vec::new(),
            errors: Vec::new(),
        };
    }
    mysql::cleaner::clean_mysql_residuals(
        app_handle,
        selected_instance,
        options.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
async fn reset_mysql_password(
    app_handle: tauri::AppHandle,
    new_password: String,
    selected_instance: Option<types::MySQLInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    access::require_admin()?;
    mysql::password_reset::reset_mysql_password(app_handle, new_password, selected_instance, override_port).await
}

#[tauri::command]
async fn change_mysql_password(
    app_handle: tauri::AppHandle,
    old_password: String,
    new_password: String,
    selected_instance: Option<types::MySQLInstance>,
    override_port: Option<u16>,
) -> Result<String, String> {
    access::require_admin()?;
    mysql::password_reset::change_mysql_password(
        app_handle,
        old_password,
        new_password,
        selected_instance,
        override_port,
    )
    .await
}

#[tauri::command]
async fn start_mysql_service(app_handle: tauri::AppHandle, service_name: String) -> Result<(), String> {
    access::require_admin()?;
    mysql::detector::start_mysql_service(app_handle, service_name).await
}

#[tauri::command]
async fn stop_mysql_service(app_handle: tauri::AppHandle, service_name: String) -> Result<(), String> {
    access::require_admin()?;
    mysql::detector::stop_mysql_service(app_handle, service_name).await
}

#[tauri::command]
async fn detect_python_versions(app_handle: tauri::AppHandle) -> Vec<PythonVersion> {
    python::detector::detect_python_versions(app_handle).await
}

#[tauri::command]
async fn detect_default_python(app_handle: tauri::AppHandle) -> Option<PythonVersion> {
    python::detector::detect_default_python(app_handle).await
}

#[tauri::command]
async fn list_python_environments(app_handle: tauri::AppHandle) -> Vec<PythonEnvironment> {
    python::env_manager::list_python_environments(app_handle).await
}

#[tauri::command]
async fn list_python_packages(app_handle: tauri::AppHandle, python_path: Option<String>) -> Vec<PythonPackage> {
    python::package_manager::list_installed_packages(app_handle, python_path).await
}

#[tauri::command]
async fn list_pip_mirrors(app_handle: tauri::AppHandle) -> Vec<PipMirror> {
    python::mirror_manager::list_pip_mirrors(app_handle).await
}

#[tauri::command]
async fn switch_pip_mirror(app_handle: tauri::AppHandle, mirror_name: String, mirror_url: String) -> Result<String, String> {
    access::require_admin()?;
    python::mirror_manager::switch_pip_mirror(app_handle, mirror_name, mirror_url).await
}

#[tauri::command]
async fn get_available_python_versions(app_handle: tauri::AppHandle) -> Result<Vec<types::AvailablePythonVersion>, String> {
    python::version_fetcher::get_available_python_versions(app_handle).await
}

#[tauri::command]
async fn download_python_only(app_handle: tauri::AppHandle, version: String, window: tauri::Window) -> Result<String, String> {
    access::require_admin()?;
    python::version_fetcher::download_python_only(app_handle, version, window).await
}

#[tauri::command]
async fn download_and_install_python(
    app_handle: tauri::AppHandle,
    version: String,
    install_path: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    access::require_admin()?;
    python::version_fetcher::download_and_install_python(app_handle, version, install_path, window).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![
        is_running_as_admin,
        set_guest_mode,
        detect_mysql,
        uninstall_mysql,
        scan_mysql_residuals,
        clean_mysql_residuals,
        reset_mysql_password,
        change_mysql_password,
        start_mysql_service,
        stop_mysql_service,
        detect_python_versions,
        detect_default_python,
        list_python_environments,
        list_python_packages,
        list_pip_mirrors,
        switch_pip_mirror,
        get_available_python_versions,
        download_python_only,
        download_and_install_python
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
