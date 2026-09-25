use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct LogMessage {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

// MySQL 相关类型
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct MySQLInstance {
    pub version: String,
    pub architecture: String,
    pub status: String,
    pub path: String,
    pub service_name: Option<String>,
    pub port: Option<u16>,
    pub is_residual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct MySQLInfo {
    pub instances: Vec<MySQLInstance>,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct MySQLPackageOption {
    /// 包类型：msi（安装包）/ zip（压缩包）
    pub package_type: String,
    pub display_name: String,
    pub download_link: String,
    #[ts(type = "number")]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct MySQLVersionInfo {
    /// 下载模式：installer（一体化管理器）/ server（独立安装包）
    pub mode: String,
    /// 所属系列分类（如 "MySQL 9.x"、"MySQL 8.0"）
    pub category: String,
    pub version: String,
    pub packages: Vec<MySQLPackageOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct CleanResult {
    pub success: bool,
    pub message: String,
    pub cleaned_items: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct ScannedPath {
    pub path: String,
    pub category: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
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

// PostgreSQL 相关类型
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PostgresqlInstance {
    pub version: String,
    pub architecture: String,
    pub status: String,
    pub path: String,
    pub service_name: Option<String>,
    pub port: Option<u16>,
    pub data_dir: Option<String>,
    pub is_residual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PostgresqlInfo {
    pub instances: Vec<PostgresqlInstance>,
    pub total_count: i32,
}

/// PostgreSQL 版本信息：单 exe 安装器，无 packages 数组
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PostgresqlVersionInfo {
    /// 完整版本号（含 build，如 "16.15-3"）
    pub version: String,
    pub download_link: String,
    #[ts(type = "number")]
    pub size: u64,
}

// Python 相关类型
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PythonVersion {
    pub version: String,
    pub path: String,
    pub executable: String,
    /// 来源：system（暂未扩充 Chocolatey/Scoop/Anaconda/pyenv-win 识别）
    pub manager: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PythonEnvironment {
    pub name: String,
    pub env_type: String,
    pub path: String,
    pub python_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PythonPackage {
    pub name: String,
    pub version: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PipMirror {
    pub name: String,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct AvailablePythonVersion {
    pub version: String,
    pub is_stable: bool,
}

// Node.js 相关类型
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct NodeVersion {
    pub version: String,
    pub path: String,
    pub executable: String,
    /// 来源：nvm / fnm / volta / system / unknown
    pub manager: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct NodeEnvironment {
    pub name: String,
    pub env_type: String,
    pub path: String,
    pub node_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct NodePackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct NpmMirror {
    pub name: String,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct AvailableNodeVersion {
    pub version: String,
    /// 是否为 LTS 版本
    pub is_lts: bool,
    /// 发布日期
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct DownloadProgress {
    /// 下载任务标识（python:{version} 或 jetbrains:{code}-{version}-{packageType}）
    pub task_id: String,
    /// 版本号（用于显示）
    pub version: String,
    #[ts(type = "number")]
    pub downloaded: u64,
    #[ts(type = "number")]
    pub total: u64,
    pub percentage: f64,
    pub status: String,
    pub completed: bool,
    pub success: bool,
    /// 是否处于暂停状态
    pub paused: bool,
}

// Java 相关类型
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct JavaVersion {
    pub version: String,
    pub path: String,
    pub executable: String,
    /// 厂商：Oracle / Adoptium / Microsoft / Amazon / Unknown
    pub vendor: String,
    /// 来源：system / JAVA_HOME
    pub manager: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct AvailableJavaVersion {
    /// 特性版本号：8 / 11 / 17 / 21 / 22 ...
    #[ts(type = "number")]
    pub feature_version: u32,
    pub is_lts: bool,
    /// 最新补丁版本号，如 "17.0.20.1+1"
    pub most_recent_version: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct MavenMirror {
    pub name: String,
    pub url: String,
    pub active: bool,
}

// JetBrains 相关类型
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct JetBrainsInstallation {
    pub product_name: String,
    pub product_code: String,
    pub version: String,
    pub install_location: String,
    pub uninstall_string: String,
    pub quiet_uninstall_string: String,
    pub is_toolbox: bool,
    pub publisher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct JetBrainsVersionInfo {
    pub product_code: String,
    pub product_name: String,
    pub version: String,
    pub date: String,
    pub download_link: String,
    #[ts(type = "number")]
    pub size: u64,
    pub checksum_link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct JetBrainsPackageOption {
    /// 包类型标识：windows / windowsZip / windowsARM64
    pub package_type: String,
    /// 显示名称
    pub display_name: String,
    pub download_link: String,
    #[ts(type = "number")]
    pub size: u64,
    pub checksum_link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct JetBrainsVersionDetail {
    pub version: String,
    pub date: String,
    /// 该版本可用的包类型列表
    pub packages: Vec<JetBrainsPackageOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct JetBrainsResidueScanResult {
    pub product_label: String,
    pub config_dirs: Vec<ScannedPath>,
    pub cache_dirs: Vec<ScannedPath>,
    pub registry_keys: Vec<String>,
    pub start_menu_shortcuts: Vec<String>,
    pub excluded_note: String,
}

/// 应用更新检查结果（Feature 18）
///
/// 后端 `check_for_updates` 命令的返回类型。
/// - `has_update`：远端最新版本号是否大于当前版本
/// - `current_version`：当前应用版本（来自 tauri.conf.json）
/// - `latest_version`：远端最新发布版本号（已去除前导 `v`）
/// - `html_url`：浏览器可访问的 release 页面 URL
/// - `body`：release 说明（changelog），可能为 None
/// - `published_at`：发布时间（ISO 8601 字符串）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub html_url: String,
    pub body: Option<String>,
    pub published_at: Option<String>,
}

/// PATH 条目（Feature 3 环境变量管理器）
///
/// 一条 PATH 项的元数据：
/// - `path`：原始路径字符串
/// - `scope`：'system'（来自 HKLM）或 'user'（来自 HKCU）
/// - `exists`：路径在文件系统中是否真实存在
/// - `tool`：识别为哪个工具（java/python/node/...），未识别为 None
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct EnvPath {
    pub path: String,
    pub scope: String,
    pub exists: bool,
    pub tool: Option<String>,
}

/// 环境变量键值对（Feature 3）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct EnvVariable {
    pub name: String,
    pub value: String,
    pub scope: String,
}

/// PATH 冲突项（Feature 3）
///
/// - `kind`：'duplicate'（重复路径）/ 'missing'（不存在）/ 'multi_version'（同工具多版本共存）
/// - `tool`：仅 multi_version 时填充工具名
/// - `paths`：涉及的路径列表
/// - `message`：人类可读说明
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct PathConflict {
    pub kind: String,
    pub tool: Option<String>,
    pub paths: Vec<String>,
    pub message: String,
}

/// 系统信息（操作系统版本、架构、CPU、内存等）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct SystemInfo {
    /// 操作系统名称，如 "Windows" / "macOS" / "Linux"
    pub os_name: Option<String>,
    /// 操作系统版本，如 "10.0.22631"
    pub os_version: Option<String>,
    /// 内核版本
    pub kernel_version: Option<String>,
    /// 主机名
    pub host_name: Option<String>,
    /// CPU 架构，如 "x86_64" / "aarch64"
    pub architecture: Option<String>,
    /// CPU 品牌名，如 "Intel(R) Core(TM) i7-12700H"
    pub cpu_name: String,
    /// CPU 主频（MHz）
    pub cpu_frequency_mhz: u64,
    /// 物理核心数
    pub cpu_physical_cores: u32,
    /// 逻辑核心数（线程数）
    pub cpu_logical_cores: u32,
    /// 总内存（字节）
    pub total_memory_bytes: u64,
    /// 已用内存（字节）
    pub used_memory_bytes: u64,
    /// 可用内存（字节）
    pub available_memory_bytes: u64,
    /// 系统运行时间（秒）
    pub uptime_seconds: u64,
}

/// 主动导出所有 TS 类型到前端 generated 目录。
/// 运行 `cargo test export_ts_types` 即可生成/更新前端类型。
/// ts-rs 的 #[ts(export)] 在程序启动时才写文件，库 crate 的 cargo build 不会触发，
/// 因此用测试显式调用 TS::export() 来在构建期生成。
#[cfg(test)]
mod ts_export {
    use super::*;
    use crate::plugin::{ToolFeature, ToolInfo};
    use ts_rs::TS;

    #[test]
    fn export_ts_types() {
        MySQLInstance::export().unwrap();
        MySQLInfo::export().unwrap();
        MySQLPackageOption::export().unwrap();
        MySQLVersionInfo::export().unwrap();
        PostgresqlInstance::export().unwrap();
        PostgresqlInfo::export().unwrap();
        PostgresqlVersionInfo::export().unwrap();
        PythonVersion::export().unwrap();
        PythonEnvironment::export().unwrap();
        PythonPackage::export().unwrap();
        PipMirror::export().unwrap();
        AvailablePythonVersion::export().unwrap();
        NodeVersion::export().unwrap();
        NodeEnvironment::export().unwrap();
        NodePackage::export().unwrap();
        NpmMirror::export().unwrap();
        AvailableNodeVersion::export().unwrap();
        DownloadProgress::export().unwrap();
        JavaVersion::export().unwrap();
        AvailableJavaVersion::export().unwrap();
        MavenMirror::export().unwrap();
        JetBrainsInstallation::export().unwrap();
        JetBrainsVersionInfo::export().unwrap();
        JetBrainsPackageOption::export().unwrap();
        JetBrainsVersionDetail::export().unwrap();
        JetBrainsResidueScanResult::export().unwrap();
        CleanResult::export().unwrap();
        ScannedPath::export().unwrap();
        CleanScanResult::export().unwrap();
        CleanOptions::export().unwrap();
        LogMessage::export().unwrap();
        ProcessOutput::export().unwrap();
        ToolFeature::export().unwrap();
        ToolInfo::export().unwrap();
        UpdateInfo::export().unwrap();
        EnvPath::export().unwrap();
        EnvVariable::export().unwrap();
        PathConflict::export().unwrap();
        SystemInfo::export().unwrap();
    }
}



