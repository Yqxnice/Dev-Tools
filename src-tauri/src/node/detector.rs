use super::super::{detector_base, logger, process_manager};
use super::super::types::NodeVersion;
use regex::Regex;
use std::sync::LazyLock;
use std::path::Path;
use std::env;
use tauri::AppHandle;

impl detector_base::RuntimeInstance for NodeVersion {
    fn get_executable(&self) -> &str { &self.executable }
    fn get_version(&self) -> &str { &self.version }
    fn get_path(&self) -> &str { &self.path }
    fn get_manager(&self) -> &str { &self.manager }
    fn get_status(&self) -> &str { &self.status }
    fn set_status(&mut self, s: String) { self.status = s; }
}

static VERSION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"v?(\d+\.\d+\.\d+)").unwrap());

fn parse_version(output: &str) -> Option<String> {
    let trimmed = output.trim();
    // node --version 输出形如 "v20.11.0"
    VERSION_REGEX
        .captures(trimmed)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// 检查指定 node.exe 路径是否有效并返回版本信息
pub(crate) async fn check_node_at_path(
    path: &str,
    manager: &str,
) -> Option<NodeVersion> {
    let node_exe = Path::new(path);

    // WindowsApps 占位符过滤
    if let Some(parent) = node_exe.parent() {
        if let Some(p) = parent.to_str() {
            if p.contains("WindowsApps") {
                return None;
            }
        }
    }

    // 占位符通常很小
    if let Ok(meta) = node_exe.metadata() {
        if meta.len() < 1024 * 10 {
            return None;
        }
    }

    let result = process_manager::execute_command(path, &["--version"]).await;
    match result {
        Ok(output) if output.exit_code == 0 => {
            let combined = format!("{} {}", output.stdout.trim(), output.stderr.trim());
            parse_version(&combined).map(|version| {
                let parent_path = node_exe
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                NodeVersion {
                    version,
                    path: parent_path,
                    executable: path.to_string(),
                    manager: manager.to_string(),
                    status: "已安装".to_string(),
                }
            })
        }
        _ => None,
    }
}

/// 检测默认 Node（PATH 中的 node）
pub async fn detect_default_node(_app_handle: AppHandle) -> Result<Option<NodeVersion>, String> {
    if let Some(v) = check_node_at_path("node", "system").await {
        return Ok(Some(v));
    }
    Ok(None)
}

/// 扫描 nvm-windows 安装的版本
async fn scan_nvm_windows(_app_handle: &AppHandle) -> Vec<NodeVersion> {
    let mut versions = Vec::new();

    // nvm-windows 通常设置 NVM_HOME 环境变量
    let nvm_home = env::var("NVM_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            dirs::home_dir()
                .map(|h| h.join("AppData").join("Roaming").join("nvm").to_string_lossy().to_string())
        });

    let Some(home) = nvm_home else { return versions; };
    let home_path = Path::new(&home);
    if !home_path.exists() || !home_path.is_dir() {
        return versions;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(home_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                // nvm-windows 版本目录形如 "v20.11.0"，内部有 node.exe
                let node_exe = entry_path.join("node.exe");
                if node_exe.exists() {
                    if let Some(p) = node_exe.to_str() {
                        if let Some(v) = check_node_at_path(p, "nvm").await {
                            versions.push(v);
                        }
                    }
                }
            }
        }
    } else {
        logger::warn(
            _app_handle,
            &format!("无法读取 NVM 目录，跳过: {}", home),
        );
    }

    versions
}

/// 扫描 fnm 安装的版本
async fn scan_fnm(_app_handle: &AppHandle) -> Vec<NodeVersion> {
    let mut versions = Vec::new();

    let fnm_dir = env::var("FNM_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            dirs::home_dir()
                .map(|h| h.join("AppData").join("Local").join("fnm").to_string_lossy().to_string())
        });

    let Some(dir) = fnm_dir else { return versions; };
    // fnm 的 node 版本通常在 <FNM_DIR>/node-versions/<version>/installation/
    let node_versions = Path::new(&dir).join("node-versions");
    if !node_versions.exists() || !node_versions.is_dir() {
        return versions;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(&node_versions).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let node_exe = entry_path.join("installation").join("node.exe");
                if node_exe.exists() {
                    if let Some(p) = node_exe.to_str() {
                        if let Some(v) = check_node_at_path(p, "fnm").await {
                            versions.push(v);
                        }
                    }
                }
            }
        }
    } else {
        logger::warn(
            _app_handle,
            &format!("无法读取 fnm 目录，跳过: {}", node_versions.display()),
        );
    }

    versions
}

/// 扫描 volta 安装的版本
async fn scan_volta(_app_handle: &AppHandle) -> Vec<NodeVersion> {
    let mut versions = Vec::new();

    let volta_home = env::var("VOLTA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            dirs::home_dir()
                .map(|h| h.join("AppData").join("Local").join("Volta").to_string_lossy().to_string())
        });

    let Some(home) = volta_home else { return versions; };
    // Volta 的 node 镜像在 <VOLTA_HOME>/tools/image/node/<version>/
    let image_dir = Path::new(&home).join("tools").join("image").join("node");
    if !image_dir.exists() || !image_dir.is_dir() {
        return versions;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(&image_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let node_exe = entry_path.join("node.exe");
                if node_exe.exists() {
                    if let Some(p) = node_exe.to_str() {
                        if let Some(v) = check_node_at_path(p, "volta").await {
                            versions.push(v);
                        }
                    }
                }
            }
        }
    } else {
        logger::warn(
            _app_handle,
            &format!("无法读取 Volta 目录，跳过: {}", image_dir.display()),
        );
    }

    versions
}

/// 从 PATH 环境变量扫描
async fn scan_from_path(_app_handle: &AppHandle) -> Vec<NodeVersion> {
    let mut versions = Vec::new();

    if let Ok(path_env) = env::var("PATH") {
        for dir in path_env.split(';').filter(|s| !s.is_empty()) {
            let node_exe = Path::new(dir).join("node.exe");
            if node_exe.exists() {
                if let Some(p) = node_exe.to_str() {
                    if let Some(v) = check_node_at_path(p, "system").await {
                        versions.push(v);
                    }
                }
            }
        }
    }

    versions
}

/// 扫描系统常见安装位置
async fn scan_system_installs(_app_handle: &AppHandle) -> Vec<NodeVersion> {
    let mut versions = Vec::new();
    let system_paths = vec![
        r"C:\Program Files\nodejs\node.exe",
        r"C:\Program Files (x86)\nodejs\node.exe",
    ];
    for p in system_paths {
        if let Some(v) = check_node_at_path(p, "system").await {
            versions.push(v);
        }
    }
    versions
}

pub async fn detect_node_versions(app_handle: AppHandle) -> Result<Vec<NodeVersion>, String> {
    logger::info(&app_handle, "开始检测 Node.js...");
    let mut versions = Vec::new();

    // 按优先级检测
    versions.extend(scan_nvm_windows(&app_handle).await);
    versions.extend(scan_fnm(&app_handle).await);
    versions.extend(scan_volta(&app_handle).await);
    versions.extend(scan_system_installs(&app_handle).await);
    versions.extend(scan_from_path(&app_handle).await);

    let versions = detector_base::dedupe_by_executable(versions);

    logger::info(
        &app_handle,
        &format!("检测完成，发现 {} 个 Node.js 版本", versions.len()),
    );
    Ok(versions)
}
