use super::super::{logger, process_manager};
use super::super::types::PythonPackage;
use tauri::AppHandle;

pub fn validate_python_path(path: &str) -> Result<(), String> {
    let lower = path.to_lowercase();
    if !lower.ends_with("python.exe") && !lower.ends_with("python3.exe") && path != "python" && path != "python3" {
        return Err("路径必须指向 python.exe 或 python3.exe".into());
    }
    if path.contains("..") {
        return Err("路径不能包含 ..".into());
    }
    Ok(())
}

pub async fn list_installed_packages(app_handle: AppHandle, python_path: Option<String>) -> Vec<PythonPackage> {
    let mut packages = Vec::new();

    let python_cmd = python_path.unwrap_or("python".to_string());
    
    if let Err(e) = validate_python_path(&python_cmd) {
        logger::error(&app_handle, &format!("Python 路径验证失败: {}", e));
        return packages;
    }
    
    let result = process_manager::execute_command(
        &python_cmd,
        &["-m", "pip", "list", "--format=json"]
    ).await;

    match result {
        Ok(output) if output.exit_code == 0 => {
            if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&output.stdout) {
                for pkg in parsed {
                    if let (Some(name), Some(version)) = (pkg["name"].as_str(), pkg["version"].as_str()) {
                        packages.push(PythonPackage {
                            name: name.to_string(),
                            version: version.to_string(),
                            summary: "".to_string(),
                        });
                    }
                }
            }
            logger::info(&app_handle, &format!("获取完成，共发现 {} 个包", packages.len()));
        }
        Ok(output) => {
            logger::warn(&app_handle, &format!("pip list 失败: {}", output.stderr));
        }
        Err(e) => {
            logger::error(&app_handle, &format!("执行失败: {}", e));
        }
    }

    packages
}
