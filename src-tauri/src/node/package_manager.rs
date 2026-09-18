use super::super::{detector_base, logger, process_manager};
use super::super::types::NodePackage;
use tauri::AppHandle;

/// Windows 上 npm 是 npm.cmd 批处理脚本，需显式指定才能正确捕获输出
fn npm_cmd() -> &'static str {
    if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    }
}

/// 验证 node 可执行文件路径
pub fn validate_node_path(path: &str) -> Result<(), String> {
    let lower = path.to_lowercase();
    if !lower.ends_with("node.exe") && path != "node" {
        return Err("路径必须指向 node.exe".into());
    }
    if path.contains("..") {
        return Err("路径不能包含 ..".into());
    }
    Ok(())
}

/// 列出全局安装的 npm 包
/// 返回 Ok 表示命令执行成功（空数组=确实没有全局包）；
/// Err 表示 node/npm 不可用或输出解析失败
pub async fn list_installed_packages(
    app_handle: AppHandle,
    node_path: Option<String>,
) -> Result<Vec<NodePackage>, String> {
    let node_cmd = node_path.unwrap_or("node".to_string());

    if let Err(e) = validate_node_path(&node_cmd) {
        let msg = format!("Node 路径验证失败: {}", e);
        logger::error(&app_handle, &msg);
        return Err(msg);
    }

    // 直接用 npm ls -g --depth=0 --json
    let result = process_manager::execute_command(
        npm_cmd(),
        &["ls", "-g", "--depth=0", "--json"],
    )
    .await;

    match result {
        Ok(output) if output.exit_code == 0 => {
            match parse_npm_ls_json(&output.stdout) {
                Ok(packages) => {
                    logger::info(
                        &app_handle,
                        &format!("获取完成，共发现 {} 个全局包", packages.len()),
                    );
                    Ok(packages)
                }
                Err(e) => {
                    let msg = format!("npm ls 输出解析失败: {}", e);
                    logger::error(&app_handle, &msg);
                    Err(msg)
                }
            }
        }
        Ok(output) => {
            // npm ls 在存在未满足依赖时会以非零退出，但仍可能输出 JSON
            if output.stdout.trim().starts_with('{') {
                match parse_npm_ls_json(&output.stdout) {
                    Ok(packages) => {
                        logger::info(
                            &app_handle,
                            &format!("获取完成（部分依赖警告），共 {} 个全局包", packages.len()),
                        );
                        return Ok(packages);
                    }
                    Err(_) => {}
                }
            }
            let stderr = output.stderr.trim();
            let msg = if stderr.is_empty() {
                format!("npm ls 执行失败，退出码: {}", output.exit_code)
            } else {
                format!("npm ls 执行失败: {}", stderr.lines().next().unwrap_or(""))
            };
            logger::warn(&app_handle, &msg);
            Err(msg)
        }
        Err(e) => {
            let msg = format!("无法执行 npm 命令（可能未安装 Node.js 或未加入 PATH）: {}", e);
            logger::error(&app_handle, &msg);
            Err(msg)
        }
    }
}

/// 解析 `npm ls -g --depth=0 --json` 的输出
fn parse_npm_ls_json(stdout: &str) -> Result<Vec<NodePackage>, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(stdout).map_err(|e| format!("JSON 解析失败: {}", e))?;

    let deps = parsed
        .get("dependencies")
        .and_then(|d| d.as_object())
        .ok_or("输出缺少 dependencies 字段")?;

    let mut packages = Vec::new();
    for (name, info) in deps {
        let version = info
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or(detector_base::ARCH_UNKNOWN)
            .to_string();
        packages.push(NodePackage {
            name: name.clone(),
            version,
        });
    }

    // 按名称排序
    packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_node_path_rejects_invalid() {
        assert!(validate_node_path("python.exe").is_err());
        assert!(validate_node_path("C:\\..\\node.exe").is_err());
    }

    #[test]
    fn validate_node_path_accepts_valid() {
        assert!(validate_node_path("node").is_ok());
        assert!(validate_node_path("C:\\Program Files\\nodejs\\node.exe").is_ok());
    }

    #[test]
    fn parse_npm_ls_json_extracts_packages() {
        let json = r#"{
          "dependencies": {
            "npm": { "version": "10.2.4" },
            "typescript": { "version": "5.3.3" },
            "pnpm": { "version": "8.14.1" }
          }
        }"#;
        let packages = parse_npm_ls_json(json).unwrap();
        assert_eq!(packages.len(), 3);
        assert!(packages.iter().any(|p| p.name == "npm" && p.version == "10.2.4"));
        assert!(packages.iter().any(|p| p.name == "typescript"));
    }

    #[test]
    fn parse_npm_ls_json_handles_empty() {
        let json = r#"{ "dependencies": {} }"#;
        let packages = parse_npm_ls_json(json).unwrap();
        assert_eq!(packages.len(), 0);
    }

    #[test]
    fn parse_npm_ls_json_rejects_missing_deps() {
        let json = r#"{ "name": "global" }"#;
        assert!(parse_npm_ls_json(json).is_err());
    }
}
