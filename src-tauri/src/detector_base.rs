//! 数据库检测通用框架
//!
//! 提取 MySQL / PostgreSQL detector 中重复的逻辑：
//! - `merge_instance` / `dedupe_instances` 实例合并去重
//! - `is_instance_valid` 残留判定
//! - `scan_directories` 常见安装目录扫描
//!
//! 各工具通过实现 `DbInstance` trait 接入通用逻辑。
//!
//! 运行时类工具（Python/Node/Java）通过实现 `RuntimeInstance` trait 接入
//! `dedupe_by_executable` / `scan_runtime_directories` 等通用函数。

use crate::{logger, service_manager};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

// ── 实例状态/字段常量：集中管理，避免魔法字符串拼写不一致 ──
pub const STATUS_NOT_INSTALLED: &str = "未安装";
pub const STATUS_NO_SERVICE: &str = "未安装服务";
pub const STATUS_RUNNING: &str = "启动";
pub const STATUS_STOPPED: &str = "停止";
pub const VERSION_UNKNOWN: &str = "未知版本";
pub const ARCH_UNKNOWN: &str = "未知";

/// 运行时实例（可执行文件型工具）通用字段访问接口
///
/// 与 `DbInstance` 区别：无 `service_name` / `port` / `is_residual` 字段，
/// 因为 Python/Node/Java 不是 Windows 服务型数据库实例。
pub trait RuntimeInstance: Clone {
    fn get_executable(&self) -> &str;
    fn get_version(&self) -> &str;
    fn get_path(&self) -> &str;
    /// 来源标签：`system` / `nvm` / `fnm` / `volta` / `JAVA_HOME` 等
    fn get_manager(&self) -> &str;
    fn get_status(&self) -> &str;
    fn set_status(&mut self, s: String);
}

/// 按 `executable` 路径去重，保留首次出现的实例
pub fn dedupe_by_executable<T: RuntimeInstance>(mut instances: Vec<T>) -> Vec<T> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out: Vec<T> = Vec::new();
    for inst in instances.drain(..) {
        if !inst.get_executable().is_empty() && seen.insert(inst.get_executable().to_string()) {
            out.push(inst);
        }
    }
    out
}

/// 通用扫描：遍历 `base` 下的子目录，对每个子目录依次尝试 `subdir_candidates` 与 `exe_names`
/// 的组合，返回命中的可执行文件完整路径列表。
///
/// `subdir_candidates` 为空切片表示直接在子目录根查找。例如：
/// - Python：`scan_runtime_directories("C:\\Python311", &["python.exe"], &[""])`
/// - Node：`scan_runtime_directories(nvm_home, &["node.exe"], &[""])`
/// - Java：`scan_runtime_directories("C:\\Program Files\\Java", &["java.exe"], &["bin"])`
pub async fn scan_runtime_directories(
    base: &str,
    exe_names: &[&str],
    subdir_candidates: &[&str],
) -> Vec<String> {
    let mut paths = Vec::new();
    let base_path = Path::new(base);
    if !base_path.exists() || !base_path.is_dir() {
        return paths;
    }
    let mut entries = match tokio::fs::read_dir(base_path).await {
        Ok(e) => e,
        Err(_) => return paths,
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        for subdir in subdir_candidates {
            let search_dir = if subdir.is_empty() {
                entry_path.clone()
            } else {
                entry_path.join(subdir)
            };
            if !search_dir.exists() {
                continue;
            }
            for exe in exe_names {
                let exe_path = search_dir.join(exe);
                if exe_path.exists() {
                    if let Some(p) = exe_path.to_str() {
                        paths.push(p.to_string());
                    }
                    break;
                }
            }
        }
    }
    paths
}

/// 数据库实例通用字段访问接口
pub trait DbInstance: Clone {
    fn get_path(&self) -> &str;
    fn set_path(&mut self, path: String);
    fn get_version(&self) -> &str;
    fn set_version(&mut self, v: String);
    fn get_architecture(&self) -> &str;
    fn set_architecture(&mut self, a: String);
    fn get_status(&self) -> &str;
    fn set_status(&mut self, s: String);
    fn get_service_name(&self) -> Option<&str>;
    fn set_service_name(&mut self, n: Option<String>);
    fn get_port(&self) -> Option<u16>;
    fn set_port(&mut self, p: Option<u16>);
    fn get_is_residual(&self) -> bool;
    fn set_is_residual(&mut self, r: bool);
}

/// 合并两个实例：用 incoming 的非空字段补全 existing
pub fn merge_instance<T: DbInstance>(existing: &mut T, incoming: T) {
    if existing.get_path().is_empty() && !incoming.get_path().is_empty() {
        existing.set_path(incoming.get_path().to_string());
    }
    if (existing.get_version().is_empty()
        || existing.get_version() == VERSION_UNKNOWN
        || existing.get_version() == ARCH_UNKNOWN)
        && !incoming.get_version().is_empty()
        && incoming.get_version() != VERSION_UNKNOWN
    {
        existing.set_version(incoming.get_version().to_string());
    }
    if (existing.get_architecture().is_empty() || existing.get_architecture() == ARCH_UNKNOWN)
        && !incoming.get_architecture().is_empty()
        && incoming.get_architecture() != ARCH_UNKNOWN
    {
        existing.set_architecture(incoming.get_architecture().to_string());
    }
    if existing.get_port().is_none() {
        existing.set_port(incoming.get_port());
    }
    if existing.get_service_name().is_none() {
        existing.set_service_name(incoming.get_service_name().map(|s| s.to_string()));
    }
    if (existing.get_status() == STATUS_NOT_INSTALLED || existing.get_status() == STATUS_NO_SERVICE)
        && incoming.get_status() != STATUS_NOT_INSTALLED
    {
        existing.set_status(incoming.get_status().to_string());
    }
}

/// 实例去重：按 path 或 service_name 合并
pub fn dedupe_instances<T: DbInstance>(mut instances: Vec<T>) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    for inst in instances.drain(..) {
        if inst.get_status() == STATUS_NOT_INSTALLED
            && inst.get_path().is_empty()
            && inst.get_service_name().is_none()
        {
            continue;
        }
        if let Some(existing) = result.iter_mut().find(|e| {
            (!inst.get_path().is_empty() && e.get_path() == inst.get_path())
                || (inst.get_service_name().is_some()
                    && e.get_service_name() == inst.get_service_name())
        }) {
            merge_instance(existing, inst);
        } else {
            result.push(inst);
        }
    }
    result
}

/// 判定实例是否有效（路径存在或服务存在），否则标记为残留
pub async fn is_instance_valid<T: DbInstance>(instance: &T) -> bool {
    let path_valid = !instance.get_path().is_empty() && Path::new(instance.get_path()).exists();
    let service_valid = if let Some(svc) = instance.get_service_name() {
        service_manager::check_service_exists(svc).await
    } else {
        false
    };
    path_valid || service_valid
}

/// 扫描常见安装目录，返回找到的 bin 目录路径列表
pub async fn scan_directories(
    common_paths: &[&str],
    executable_names: &[&str],
) -> Vec<String> {
    let mut paths = Vec::new();
    for base_path in common_paths {
        let base = Path::new(base_path);
        if base.exists() && base.is_dir() {
            if let Ok(mut entries) = tokio::fs::read_dir(base).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        let bin_path = path.join("bin");
                        if bin_path.exists() {
                            let found = executable_names.iter().any(|name| {
                                bin_path.join(name).exists()
                            });
                            if found {
                                paths.push(bin_path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    paths
}

/// 从 bin 目录选择可执行文件（按优先级返回第一个存在的）
pub fn select_executable(bin_path: &str, names: &[&str]) -> Option<PathBuf> {
    let bin_dir = Path::new(bin_path);
    names.iter().find_map(|name| {
        let exe = bin_dir.join(name);
        if exe.exists() {
            Some(exe)
        } else {
            None
        }
    })
}

/// 验证所有实例并标记残留状态
pub async fn validate_instances<T: DbInstance>(instances: Vec<T>) -> Vec<T> {
    let mut validated = Vec::new();
    for mut instance in instances {
        let is_valid = is_instance_valid(&instance).await;
        instance.set_is_residual(!is_valid);
        validated.push(instance);
    }
    validated
}

/// 通用日志辅助
pub fn log_detect_start(app: Option<&AppHandle>, tool_name: &str) {
    if let Some(handle) = app {
        logger::info(handle, &format!("开始检测 {}...", tool_name));
    }
}

pub fn log_detect_complete(app: Option<&AppHandle>, tool_name: &str, count: usize) {
    if let Some(handle) = app {
        logger::info(
            handle,
            &format!("检测完成，发现 {} 个 {} 实例", count, tool_name),
        );
    }
}

/// 归一化路径用于比较：去掉引号/尾随分隔符，统一小写。
fn normalize_path(p: &str) -> String {
    p.trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

/// 两个路径是否指向同一位置（大小写、尾随 `\`、引号不敏感）。
fn same_path(a: &str, b: &str) -> bool {
    !a.trim().is_empty() && normalize_path(a) == normalize_path(b)
}

/// 校验「用户前端选中、但探测结果里找不到对应项」的实例。
///
/// 该实例随后会被用于 `net stop <service>`、拼接 `psql.exe`/`mysql.exe` 等操作，
/// 服务名与安装路径必须先过真实校验，不能直接信任 IPC 输入。
fn validate_selected_instance<T: DbInstance>(sel: &T, tool_name: &str) -> Result<(), String> {
    if normalize_path(sel.get_path()).is_empty() {
        return Err(format!("所选 {} 实例缺少安装路径", tool_name));
    }
    match sel.get_service_name() {
        Some(service) => crate::process_manager::validate_service_name(service)
            .map_err(|e| format!("所选 {} 实例服务名非法: {}", tool_name, e)),
        None => Err(format!("所选 {} 实例没有服务名，无法操作", tool_name)),
    }
}

/// 选择目标实例：优先使用用户选中的，否则取第一个同时有路径和服务名的实例。
///
/// 前端选中的实例只是「候选」：先尝试在实际探测结果 `instances` 中按安装路径
/// 找回权威数据；找不到时才使用前端数据，且必须先通过
/// [`validate_selected_instance`] 的路径/服务名校验。
pub fn select_valid_instance<'a, T: DbInstance>(
    selected: Option<&'a T>,
    instances: &'a [T],
    tool_name: &str,
) -> Result<&'a T, String> {
    if let Some(sel) = selected {
        if let Some(detected) = instances.iter().find(|i| same_path(i.get_path(), sel.get_path())) {
            return Ok(detected);
        }
        validate_selected_instance(sel, tool_name)?;
        return Ok(sel);
    }
    instances
        .iter()
        .find(|inst| !inst.get_path().is_empty() && inst.get_service_name().is_some())
        .ok_or_else(|| {
            format!(
                "未找到有效的 {} 实例（需要有安装路径和服务名）",
                tool_name
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Default)]
    struct Fake {
        path: String,
        version: String,
        architecture: String,
        status: String,
        service_name: Option<String>,
        port: Option<u16>,
        is_residual: bool,
    }

    impl Fake {
        fn new(path: &str, service: Option<&str>) -> Self {
            Self {
                path: path.to_string(),
                version: "8.0".to_string(),
                architecture: "x64".to_string(),
                status: "stopped".to_string(),
                service_name: service.map(str::to_string),
                port: Some(3306),
                is_residual: false,
            }
        }
    }

    impl DbInstance for Fake {
        fn get_path(&self) -> &str {
            &self.path
        }
        fn set_path(&mut self, path: String) {
            self.path = path;
        }
        fn get_version(&self) -> &str {
            &self.version
        }
        fn set_version(&mut self, v: String) {
            self.version = v;
        }
        fn get_architecture(&self) -> &str {
            &self.architecture
        }
        fn set_architecture(&mut self, a: String) {
            self.architecture = a;
        }
        fn get_status(&self) -> &str {
            &self.status
        }
        fn set_status(&mut self, s: String) {
            self.status = s;
        }
        fn get_service_name(&self) -> Option<&str> {
            self.service_name.as_deref()
        }
        fn set_service_name(&mut self, n: Option<String>) {
            self.service_name = n;
        }
        fn get_port(&self) -> Option<u16> {
            self.port
        }
        fn set_port(&mut self, p: Option<u16>) {
            self.port = p;
        }
        fn get_is_residual(&self) -> bool {
            self.is_residual
        }
        fn set_is_residual(&mut self, r: bool) {
            self.is_residual = r;
        }
    }

    #[test]
    fn path_comparison_ignores_case_quotes_and_trailing_slash() {
        assert!(same_path(
            r"C:\Program Files\MySQL\MySQL Server 8.0\",
            r#""c:\program files\mysql\mysql server 8.0""#
        ));
        assert!(!same_path(r"C:\a", r"C:\b"));
        assert!(!same_path("", r"C:\a"));
        assert!(!same_path("   ", r"C:\a"));
    }

    #[test]
    fn selected_is_returned_when_no_detected_match() {
        let sel = Fake::new(r"D:\MySQL", Some("MySQL80"));
        let detected = vec![Fake::new(r"C:\MySQL", Some("MySQL57"))];
        let picked = select_valid_instance(Some(&sel), &detected, "MySQL").unwrap();
        assert_eq!(picked.get_path(), r"D:\MySQL");
    }

    #[test]
    fn detected_instance_overrides_same_path_selection() {
        let sel = Fake::new(r"C:\MySQL\", Some("SpoofedService"));
        let detected = vec![Fake::new(r"C:\MySQL", Some("MySQL80"))];
        let picked = select_valid_instance(Some(&sel), &detected, "MySQL").unwrap();
        assert_eq!(picked.get_service_name(), Some("MySQL80"));
    }

    #[test]
    fn selected_without_path_is_rejected() {
        let sel = Fake::new("   ", Some("MySQL80"));
        let err = select_valid_instance(Some(&sel), &[], "MySQL").unwrap_err();
        assert!(err.contains("缺少安装路径"), "{}", err);
    }

    #[test]
    fn selected_without_service_is_rejected() {
        let sel = Fake::new(r"D:\MySQL", None);
        let err = select_valid_instance(Some(&sel), &[], "MySQL").unwrap_err();
        assert!(err.contains("没有服务名"), "{}", err);
    }

    #[test]
    fn selected_with_illegal_service_name_is_rejected() {
        let sel = Fake::new(r"D:\MySQL", Some("MySQL80 & net user evil /add"));
        let err = select_valid_instance(Some(&sel), &[], "MySQL").unwrap_err();
        assert!(err.contains("服务名非法"), "{}", err);
    }

    #[test]
    fn fallback_picks_first_detected_instance_with_path_and_service() {
        let detected = vec![
            Fake::new("", None),
            Fake::new(r"C:\MySQL", None),
            Fake::new(r"C:\MySQL2", Some("MySQL81")),
            Fake::new(r"C:\MySQL3", Some("MySQL82")),
        ];
        let picked = select_valid_instance(None, &detected, "MySQL").unwrap();
        assert_eq!(picked.get_service_name(), Some("MySQL81"));
    }

    #[test]
    fn fallback_errors_when_no_valid_instance() {
        let detected = vec![Fake::new(r"C:\MySQL", None)];
        let err = select_valid_instance(None, &detected, "MySQL").unwrap_err();
        assert!(err.contains("未找到有效的 MySQL 实例"), "{}", err);
    }
}
