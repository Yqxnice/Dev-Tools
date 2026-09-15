use super::super::{logger, process_manager};
use super::super::types::PipMirror;
use super::package_manager::validate_python_path;
use tauri::AppHandle;

pub fn validate_mirror_url(url: &str) -> Result<(), String> {
    process_manager::validate_mirror_url(url).map_err(|e| e.to_user_message())
}

pub fn get_default_mirrors() -> Vec<PipMirror> {
    vec![
        PipMirror {
            name: "官方源".to_string(),
            url: "https://pypi.org/simple/".to_string(),
            active: true,
        },
        PipMirror {
            name: "清华大学".to_string(),
            url: "https://pypi.tuna.tsinghua.edu.cn/simple/".to_string(),
            active: false,
        },
        PipMirror {
            name: "阿里云".to_string(),
            url: "https://mirrors.aliyun.com/pypi/simple/".to_string(),
            active: false,
        },
        PipMirror {
            name: "中国科技大学".to_string(),
            url: "https://pypi.mirrors.ustc.edu.cn/simple/".to_string(),
            active: false,
        },
        PipMirror {
            name: "豆瓣".to_string(),
            url: "https://pypi.douban.com/simple/".to_string(),
            active: false,
        },
    ]
}

/// 解析 pip 命令对应的 Python 解释器：优先使用用户选中的 python.exe，否则回退 PATH 中的 python
fn resolve_python_cmd(python_path: Option<String>) -> Result<String, String> {
    let cmd = python_path.unwrap_or_else(|| "python".to_string());
    validate_python_path(&cmd).map_err(|e| format!("Python 路径验证失败: {}", e))?;
    Ok(cmd)
}

pub async fn list_pip_mirrors(app_handle: AppHandle, python_path: Option<String>) -> Result<Vec<PipMirror>, String> {
    let mut mirrors = get_default_mirrors();

    match resolve_python_cmd(python_path) {
        Ok(python_cmd) => {
            if let Some(active_url) = get_current_mirror(&python_cmd).await {
                for mirror in &mut mirrors {
                    mirror.active = mirror.url == active_url;
                }
            } else {
                logger::warn(&app_handle, "未能读取当前 pip 镜像配置（可能未配置过或该解释器无 pip）");
            }
        }
        Err(e) => logger::warn(&app_handle, &e),
    }

    Ok(mirrors)
}

async fn get_current_mirror(python_cmd: &str) -> Option<String> {
    let result = process_manager::execute_command(
        python_cmd,
        &["-m", "pip", "config", "get", "global.index-url"],
    )
    .await;
    match result {
        Ok(output) if output.exit_code == 0 => {
            let stdout = output.stdout.trim();
            if stdout.is_empty() {
                None
            } else {
                Some(stdout.to_string())
            }
        }
        _ => None,
    }
}

pub async fn switch_pip_mirror(
    app_handle: AppHandle,
    mirror_name: String,
    mirror_url: String,
    python_path: Option<String>,
) -> Result<String, String> {
    validate_mirror_url(&mirror_url)?;
    let python_cmd = resolve_python_cmd(python_path)?;
    logger::info(
        &app_handle,
        &format!("通过解释器 {} 切换 pip 镜像到 {}", python_cmd, mirror_name),
    );
    let result = process_manager::execute_command(
        &python_cmd,
        &["-m", "pip", "config", "set", "global.index-url", &mirror_url],
    )
    .await;

    match result {
        Ok(output) if output.exit_code == 0 => {
            logger::info(&app_handle, &format!("已切换到: {}", mirror_name));
            Ok(format!("已成功切换到 {}", mirror_name))
        }
        Ok(output) => {
            let err_msg = format!("切换失败: {}", output.stderr);
            logger::error(&app_handle, &err_msg);
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = format!("执行命令失败: {}", e);
            logger::error(&app_handle, &err_msg);
            Err(err_msg)
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
    fn validate_mirror_url_rejects_invalid_scheme() {
        assert!(validate_mirror_url("ftp://mirror.example.com").is_err());
    }

    #[test]
    fn validate_mirror_url_rejects_special_chars() {
        assert!(validate_mirror_url("https://example.com;rm").is_err());
        assert!(validate_mirror_url("https://example.com|ls").is_err());
        assert!(validate_mirror_url("https://example.com&whoami").is_err());
        assert!(validate_mirror_url("https://example.com`id`").is_err());
    }

    #[test]
    fn validate_mirror_url_accepts_valid() {
        assert!(validate_mirror_url("https://pypi.org/simple/").is_ok());
        assert!(validate_mirror_url("http://mirrors.aliyun.com/pypi/simple/").is_ok());
    }

    #[test]
    fn get_default_mirrors_has_five_entries() {
        let mirrors = get_default_mirrors();
        assert_eq!(mirrors.len(), 5);
        assert!(mirrors.iter().any(|m| m.name == "官方源"));
        assert!(mirrors.iter().any(|m| m.name == "清华大学"));
        assert!(mirrors.iter().any(|m| m.name == "阿里云"));
    }

    #[test]
    fn default_mirror_official_is_active() {
        let mirrors = get_default_mirrors();
        let official = mirrors.iter().find(|m| m.name == "官方源").unwrap();
        assert!(official.active);
    }

    #[test]
    fn default_mirrors_urls_are_valid() {
        for mirror in get_default_mirrors() {
            assert!(mirror.url.starts_with("http"), "{} url invalid", mirror.name);
        }
    }
}
