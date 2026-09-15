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

/// 返回 Result：Ok 表示 pip 命令成功执行（空数组=确实没有包），
/// Err 表示 Python/pip 不可用或输出无法解析（与"未安装包"明确区分，错误信息可直接展示）
pub async fn list_installed_packages(app_handle: AppHandle, python_path: Option<String>) -> Result<Vec<PythonPackage>, String> {
    let mut packages = Vec::new();

    let python_cmd = python_path.unwrap_or("python".to_string());

    if let Err(e) = validate_python_path(&python_cmd) {
        let msg = format!("Python 路径验证失败: {}", e);
        logger::error(&app_handle, &msg);
        return Err(msg);
    }

    let result = process_manager::execute_command(
        &python_cmd,
        &["-m", "pip", "list", "--format=json"]
    ).await;

    match result {
        Ok(output) if output.exit_code == 0 => {
            match serde_json::from_str::<Vec<serde_json::Value>>(&output.stdout) {
                Ok(parsed) => {
                    for pkg in parsed {
                        if let (Some(name), Some(version)) = (pkg["name"].as_str(), pkg["version"].as_str()) {
                            packages.push(PythonPackage {
                                name: name.to_string(),
                                version: version.to_string(),
                                summary: "".to_string(),
                            });
                        }
                    }
                    logger::info(&app_handle, &format!("获取完成，共发现 {} 个包", packages.len()));
                    Ok(packages)
                }
                Err(e) => {
                    let msg = format!("pip list 输出解析失败: {}（原始输出: {}）", e, output.stdout.trim());
                    logger::error(&app_handle, &msg);
                    Err(msg)
                }
            }
        }
        Ok(output) => {
            let stderr = output.stderr.trim();
            let msg = if stderr.is_empty() {
                format!("pip list 执行失败，退出码: {}", output.exit_code)
            } else {
                format!("pip list 执行失败: {}", stderr)
            };
            logger::warn(&app_handle, &msg);
            Err(msg)
        }
        Err(e) => {
            let msg = format!("无法执行 Python/pip 命令（可能未安装 Python 或未加入 PATH）: {}", e);
            logger::error(&app_handle, &msg);
            Err(msg)
        }
    }
}
