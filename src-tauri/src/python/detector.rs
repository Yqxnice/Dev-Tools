use super::super::{detector_base, logger, process_manager};
use super::super::types::PythonVersion;
use regex::Regex;
use std::sync::LazyLock;
use std::path::Path;
use tauri::AppHandle;
use std::env;
use futures_util::future::join_all;

/// Python 版本统一来源标签：当前实现未识别 Chocolatey/Scoop/Anaconda/pyenv-win
const PYTHON_MANAGER: &str = "system";

impl detector_base::RuntimeInstance for PythonVersion {
    fn get_executable(&self) -> &str { &self.executable }
    fn get_version(&self) -> &str { &self.version }
    fn get_path(&self) -> &str { &self.path }
    fn get_manager(&self) -> &str { &self.manager }
    fn get_status(&self) -> &str { &self.status }
    fn set_status(&mut self, s: String) { self.status = s; }
}

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Python (\d+\.\d+\.\d+)").unwrap());

fn parse_version(output: &str) -> Option<String> {
    if let Some(captures) = VERSION_REGEX.captures(output) {
        captures.get(1).map(|m| m.as_str().to_string())
    } else {
        None
    }
}

pub(crate) async fn check_python_at_path(path: &str) -> Option<PythonVersion> {
    let python_exe = Path::new(path);
    if !python_exe.exists() {
        return None;
    }

    // 过滤掉 WindowsApps 中的占位符
    if let Some(parent) = python_exe.parent() {
        if let Some(parent_str) = parent.to_str() {
            if parent_str.contains("WindowsApps") {
                return None;
            }
        }
    }

    // 检查文件大小，占位符通常很小
    if let Ok(metadata) = python_exe.metadata() {
        if metadata.len() < 1024 * 10 { // 小于 10KB 很可能是占位符
            return None;
        }
    }

    // 先获取真实的可执行文件路径（解析符号链接）
    let real_path_result = process_manager::execute_command(
        path, 
        &["-c", "import sys; print(sys.executable)"]
    ).await;
    
    let (real_executable, real_parent) = match real_path_result {
        Ok(output) if output.exit_code == 0 && !output.stdout.trim().is_empty() => {
            let real_exe = output.stdout.trim().to_string();
            let real_path_obj = Path::new(&real_exe);
            let real_parent_str = real_path_obj.parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();
            (real_exe, real_parent_str)
        },
        _ => {
            // 如果无法获取真实路径，使用原始路径
            let parent_path = python_exe.parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();
            (path.to_string(), parent_path)
        }
    };

    // 再获取版本号
    let result = process_manager::execute_command(path, &["--version"]).await;
    match result {
        Ok(output) if output.exit_code == 0 => {
            let version_str = output.stdout.trim().to_string() + " " + output.stderr.trim();
            parse_version(&version_str).map(|version| PythonVersion {
                version,
                path: real_parent,
                executable: real_executable,
                manager: PYTHON_MANAGER.to_string(),
                status: "已安装".to_string(),
            })
        }
        _ => None,
    }
}

async fn try_detect_with_py(_app_handle: &AppHandle) -> Vec<PythonVersion> {
    let mut versions = Vec::new();
    
    // 尝试使用 py launcher 列出已安装的 Python
    let result = process_manager::execute_command("py", &["-0p"]).await;
    if let Ok(output) = result {
        if output.exit_code == 0 {
            let mut futures = Vec::new();
            for minor in 6..=14 {
                let version_arg = format!("-3.{}", minor);
                futures.push(async move {
                    let path_result = process_manager::execute_command("py", &[&version_arg, "-c", "import sys; print(sys.executable)"]).await;
                    if let Ok(path_out) = path_result {
                        if path_out.exit_code == 0 {
                            let exe_path = path_out.stdout.trim().to_string();
                            if !exe_path.is_empty() {
                                return check_python_at_path(&exe_path).await;
                            }
                        }
                    }
                    None
                });
            }
            let results = join_all(futures).await;
            for result in results.into_iter().flatten() {
                versions.push(result);
            }
        }
    }
    
    versions
}

async fn try_detect_from_path(_app_handle: &AppHandle) -> Vec<PythonVersion> {
    let mut versions = Vec::new();
    
    // 从 PATH 环境变量中查找
    if let Ok(path_env) = env::var("PATH") {
        for dir in path_env.split(';').filter(|s| !s.is_empty()) {
            let python_exe = Path::new(dir).join("python.exe");
            if python_exe.exists() {
                if let Some(path_str) = python_exe.to_str() {
                    if let Some(py_ver) = check_python_at_path(path_str).await {
                        versions.push(py_ver);
                    }
                }
            }
            
            let python3_exe = Path::new(dir).join("python3.exe");
            if python3_exe.exists() {
                if let Some(path_str) = python3_exe.to_str() {
                    if let Some(py_ver) = check_python_at_path(path_str).await {
                        versions.push(py_ver);
                    }
                }
            }
        }
    }
    
    versions
}

async fn try_detect_from_appdata(app_handle: &AppHandle) -> Vec<PythonVersion> {
    let mut versions = Vec::new();

    // 检查用户目录下的 Python
    if let Some(user_dir) = dirs::home_dir() {
        let appdata_local = user_dir.join("AppData").join("Local").join("Programs").join("Python");

        if appdata_local.exists() && appdata_local.is_dir() {
            match tokio::fs::read_dir(appdata_local).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            let python_exe = entry_path.join("python.exe");
                            if python_exe.exists() {
                                if let Some(path_str) = python_exe.to_str() {
                                    if let Some(py_ver) = check_python_at_path(path_str).await {
                                        versions.push(py_ver);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => logger::warn(
                    app_handle,
                    &format!("扫描 AppData Python 目录失败，跳过: {}", e),
                ),
            }
        }
    }

    versions
}

pub async fn detect_default_python(_app_handle: AppHandle) -> Result<Option<PythonVersion>, String> {
    // 1. 先尝试 python
    if let Some(py_ver) = check_python_at_path("python").await {
        return Ok(Some(py_ver));
    }

    // 2. 再尝试 python3
    if let Some(py_ver) = check_python_at_path("python3").await {
        return Ok(Some(py_ver));
    }

    Ok(None)
}

pub async fn detect_python_versions(app_handle: AppHandle) -> Result<Vec<PythonVersion>, String> {
    logger::info(&app_handle, "开始检测 Python...");
    let mut versions = Vec::new();

    // 1. 先尝试检测环境变量中的 python 和 python3
    let common_paths = vec![
        "python",
        "python3",
    ];

    for path in common_paths {
        if let Some(py_ver) = check_python_at_path(path).await {
            versions.push(py_ver);
        }
    }

    // 2. 使用 py launcher 检测
    let py_versions = try_detect_with_py(&app_handle).await;
    versions.extend(py_versions);

    // 3. 从 PATH 环境变量检测
    let path_versions = try_detect_from_path(&app_handle).await;
    versions.extend(path_versions);

    // 4. 从用户目录检测
    let appdata_versions = try_detect_from_appdata(&app_handle).await;
    versions.extend(appdata_versions);

    // 5. 检测常见安装位置（备选）
    let system_paths = vec![
        r"C:\Python312\python.exe",
        r"C:\Python311\python.exe",
        r"C:\Python310\python.exe",
        r"C:\Python39\python.exe",
        r"C:\Python38\python.exe",
        r"C:\Program Files\Python312\python.exe",
        r"C:\Program Files\Python311\python.exe",
        r"C:\Program Files\Python310\python.exe",
        r"C:\Program Files\Python39\python.exe",
        r"C:\Program Files\Python38\python.exe",
        r"C:\Program Files (x86)\Python312\python.exe",
        r"C:\Program Files (x86)\Python311\python.exe",
        r"C:\Program Files (x86)\Python310\python.exe",
        r"C:\Program Files (x86)\Python39\python.exe",
        r"C:\Program Files (x86)\Python38\python.exe",
    ];

    for path in system_paths {
        if let Some(py_ver) = check_python_at_path(path).await {
            versions.push(py_ver);
        }
    }

    let versions = detector_base::dedupe_by_executable(versions);

    logger::info(&app_handle, &format!("检测完成，发现 {} 个 Python 版本", versions.len()));

    Ok(versions)
}
