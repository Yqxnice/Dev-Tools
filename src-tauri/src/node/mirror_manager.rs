use super::super::{logger, process_manager};
use super::super::types::NpmMirror;
use tauri::AppHandle;

/// Windows 上 npm 是 npm.cmd 批处理脚本，需显式指定才能正确捕获输出
fn npm_cmd() -> &'static str {
    if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    }
}

pub fn validate_mirror_url(url: &str) -> Result<(), String> {
    process_manager::validate_mirror_url(url).map_err(|e| e.to_user_message())
}

pub fn get_default_mirrors() -> Vec<NpmMirror> {
    vec![
        NpmMirror {
            name: "官方源".to_string(),
            url: "https://registry.npmjs.org/".to_string(),
            active: true,
        },
        NpmMirror {
            name: "阿里云".to_string(),
            url: "https://registry.npmmirror.com".to_string(),
            active: false,
        },
        NpmMirror {
            name: "腾讯云".to_string(),
            url: "https://mirrors.cloud.tencent.com/npm/".to_string(),
            active: false,
        },
        NpmMirror {
            name: "华为云".to_string(),
            url: "https://repo.huaweicloud.com/repository/npm/".to_string(),
            active: false,
        },
    ]
}

/// 规范化 registry URL：去掉末尾斜杠便于比较
fn normalize_registry(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// 读取当前生效的 registry
async fn get_current_registry() -> Option<String> {
    let result = process_manager::execute_command(npm_cmd(), &["config", "get", "registry"]).await;
    match result {
        Ok(output) if output.exit_code == 0 => {
            let url = output.stdout.trim();
            // npm 未设置时输出 "undefined"
            if url.is_empty() || url == "undefined" {
                Some(normalize_registry("https://registry.npmjs.org/"))
            } else {
                Some(normalize_registry(url))
            }
        }
        _ => None,
    }
}

pub async fn list_npm_mirrors(app_handle: AppHandle) -> Result<Vec<NpmMirror>, String> {
    let mut mirrors = get_default_mirrors();

    match get_current_registry().await {
        Some(active_url) => {
            for mirror in &mut mirrors {
                mirror.active = active_url == normalize_registry(&mirror.url);
            }
        }
        None => {
            logger::warn(&app_handle, "未能读取当前 npm registry 配置，显示默认列表");
        }
    }

    Ok(mirrors)
}

/// 永久切换 npm 镜像源（写入用户级 ~/.npmrc）
pub async fn switch_npm_mirror(
    app_handle: AppHandle,
    mirror_name: String,
    mirror_url: String,
) -> Result<String, String> {
    validate_mirror_url(&mirror_url)?;
    logger::info(
        &app_handle,
        &format!("切换 npm registry 到 {}", mirror_name),
    );

    let result = process_manager::execute_command(
        npm_cmd(),
        &["config", "set", "registry", &mirror_url],
    )
    .await;

    match result {
        Ok(output) if output.exit_code == 0 => {
            logger::info(&app_handle, &format!("已切换到: {}", mirror_name));
            Ok(format!("已成功切换到 {}", mirror_name))
        }
        Ok(output) => {
            let stderr = output.stderr.trim();
            let msg = if stderr.is_empty() {
                format!("切换失败，退出码: {}", output.exit_code)
            } else {
                format!("切换失败: {}", stderr)
            };
            logger::error(&app_handle, &msg);
            Err(msg)
        }
        Err(e) => {
            let msg = format!("执行 npm 命令失败: {}", e);
            logger::error(&app_handle, &msg);
            Err(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_mirror_url_rejects_empty() {
        assert!(validate_mirror_url("").is_err());
    }

    #[test]
    fn validate_mirror_url_accepts_valid() {
        assert!(validate_mirror_url("https://registry.npmmirror.com/").is_ok());
        assert!(validate_mirror_url("https://registry.npmjs.org").is_ok());
    }

    #[test]
    fn get_default_mirrors_has_four_entries() {
        let mirrors = get_default_mirrors();
        assert_eq!(mirrors.len(), 4);
        assert!(mirrors.iter().any(|m| m.name == "官方源"));
        assert!(mirrors.iter().any(|m| m.name == "阿里云"));
        assert!(mirrors.iter().any(|m| m.name == "腾讯云"));
        assert!(mirrors.iter().any(|m| m.name == "华为云"));
    }

    #[test]
    fn official_mirror_is_active_by_default() {
        let mirrors = get_default_mirrors();
        let official = mirrors.iter().find(|m| m.name == "官方源").unwrap();
        assert!(official.active);
    }

    #[test]
    fn normalize_registry_strips_trailing_slash() {
        assert_eq!(normalize_registry("https://registry.npmjs.org/"), "https://registry.npmjs.org");
        assert_eq!(normalize_registry("https://registry.npmmirror.com"), "https://registry.npmmirror.com");
    }
}
