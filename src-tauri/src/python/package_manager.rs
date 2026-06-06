use super::super::{logger, process_manager};
use super::super::types::PythonPackage;
use tauri::AppHandle;

pub async fn list_installed_packages(app_handle: AppHandle, python_path: Option<String>) -> Vec<PythonPackage> {
    let mut packages = Vec::new();

    let python_cmd = python_path.unwrap_or("python".to_string());
    
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
