use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

// MySQL 相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLInstance {
    pub version: String,
    pub architecture: String,
    pub status: String,
    pub path: String,
    pub service_name: Option<String>,
    pub port: Option<u16>,
    pub is_residual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLInfo {
    pub instances: Vec<MySQLInstance>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub success: bool,
    pub message: String,
    pub cleaned_items: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedPath {
    pub path: String,
    pub category: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanScanResult {
    pub instance_label: String,
    pub selected_version: String,
    pub services: Vec<String>,
    pub directories: Vec<ScannedPath>,
    pub registry_keys: Vec<String>,
    pub start_menu_shortcuts: Vec<String>,
    pub path_entries: Vec<String>,
    pub excluded_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanOptions {
    pub kill_processes: bool,
    pub remove_services: bool,
    pub clean_install_dir: bool,
    pub clean_program_data: bool,
    pub clean_registry_uninstall: bool,
    pub clean_registry_mysql_ab: bool,
    pub clean_registry_services: bool,
    pub clean_registry_installer: bool,
    pub clean_start_menu: bool,
    pub clean_path: bool,
    pub clean_odbc: bool,
    pub clean_user_registry: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            kill_processes: true,
            remove_services: true,
            clean_install_dir: true,
            clean_program_data: true,
            clean_registry_uninstall: true,
            clean_registry_mysql_ab: true,
            clean_registry_services: true,
            clean_registry_installer: true,
            clean_start_menu: true,
            clean_path: false,
            clean_odbc: false,
            clean_user_registry: false,
        }
    }
}

// Python 相关类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonVersion {
    pub version: String,
    pub path: String,
    pub executable: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEnvironment {
    pub name: String,
    pub env_type: String,
    pub path: String,
    pub python_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonPackage {
    pub name: String,
    pub version: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipMirror {
    pub name: String,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailablePythonVersion {
    pub version: String,
    pub is_stable: bool,
    pub release_date: Option<String>,
    pub download_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub version: String,
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub version: String,
    pub phase: String,
    pub message: String,
    pub percentage: u32,
    pub completed: bool,
    pub success: bool,
    pub error: Option<String>,
}
