use super::super::{detector_base, logger};
use super::super::types::PythonEnvironment;
use super::detector;
use std::path::Path;
use tauri::AppHandle;

async fn get_python_version_from_path(python_path: &str) -> String {
    if let Some(ver) = detector::check_python_at_path(python_path).await {
        ver.version
    } else {
        detector_base::ARCH_UNKNOWN.to_string()
    }
}

async fn scan_virtual_environments(app_handle: &AppHandle) -> Vec<PythonEnvironment> {
    let mut envs = Vec::new();

    let common_locations = vec![
        dirs::home_dir().map(|p| p.join(".venvs")),
        dirs::home_dir().map(|p| p.join(".virtualenvs")),
        dirs::home_dir().map(|p| p.join("venv")),
        dirs::home_dir().map(|p| p.join("Envs")),
    ];

    for location in common_locations.into_iter().flatten() {
        if location.exists() && location.is_dir() {
            match tokio::fs::read_dir(&location).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            if let Some(env) = check_virtual_env(&entry_path, app_handle).await {
                                envs.push(env);
                            }
                        }
                    }
                }
                Err(e) => logger::warn(
                    app_handle,
                    &format!("扫描虚拟环境目录 {} 失败，跳过: {}", location.display(), e),
                ),
            }
        }
    }

    envs
}

async fn check_virtual_env(env_path: &Path, _app_handle: &AppHandle) -> Option<PythonEnvironment> {
    let python_exe = if cfg!(windows) {
        env_path.join("Scripts").join("python.exe")
    } else {
        env_path.join("bin").join("python")
    };

    if python_exe.exists() {
        let name = env_path.file_name()?.to_str()?.to_string();
        let version = get_python_version_from_path(python_exe.to_str()?).await;
        
        Some(PythonEnvironment {
            name,
            env_type: "虚拟环境".to_string(),
            path: env_path.to_str()?.to_string(),
            python_version: version,
        })
    } else {
        None
    }
}

pub async fn list_python_environments(app_handle: AppHandle) -> Result<Vec<PythonEnvironment>, String> {
    let mut envs = Vec::new();

    let system_pythons = match detector::detect_python_versions(app_handle.clone()).await {
        Ok(v) => v,
        Err(e) => {
            logger::warn(&app_handle, &format!("检测系统 Python 失败，跳过: {}", e));
            Vec::new()
        }
    };
    for py in system_pythons {
        envs.push(PythonEnvironment {
            name: format!("系统 Python {}", py.version),
            env_type: "系统".to_string(),
            path: py.path,
            python_version: py.version,
        });
    }

    let virtual_envs = scan_virtual_environments(&app_handle).await;
    envs.extend(virtual_envs);

    logger::info(&app_handle, &format!("扫描完成，共发现 {} 个 Python 环境", envs.len()));
    Ok(envs)
}
