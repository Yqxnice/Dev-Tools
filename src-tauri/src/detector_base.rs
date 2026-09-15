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
        || existing.get_version() == "未知版本"
        || existing.get_version() == "未知")
        && !incoming.get_version().is_empty()
        && incoming.get_version() != "未知版本"
    {
        existing.set_version(incoming.get_version().to_string());
    }
    if (existing.get_architecture().is_empty() || existing.get_architecture() == "未知")
        && !incoming.get_architecture().is_empty()
        && incoming.get_architecture() != "未知"
    {
        existing.set_architecture(incoming.get_architecture().to_string());
    }
    if existing.get_port().is_none() {
        existing.set_port(incoming.get_port());
    }
    if existing.get_service_name().is_none() {
        existing.set_service_name(incoming.get_service_name().map(|s| s.to_string()));
    }
    if (existing.get_status() == "未安装" || existing.get_status() == "未安装服务")
        && incoming.get_status() != "未安装"
    {
        existing.set_status(incoming.get_status().to_string());
    }
}

/// 实例去重：按 path 或 service_name 合并
pub fn dedupe_instances<T: DbInstance>(mut instances: Vec<T>) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    for inst in instances.drain(..) {
        if inst.get_status() == "未安装"
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

/// 选择目标实例：优先使用用户选中的，否则取第一个同时有路径和服务名的实例
pub fn select_valid_instance<'a, T: DbInstance>(
    selected: Option<&'a T>,
    instances: &'a [T],
    tool_name: &str,
) -> Result<&'a T, String> {
    if let Some(inst) = selected {
        return Ok(inst);
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
