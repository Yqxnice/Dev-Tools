use super::super::{logger, process_manager};
use super::super::types::PipMirror;
use tauri::AppHandle;

pub fn validate_mirror_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("镜像源 URL 不能为空".into());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("镜像源 URL 必须以 http:// 或 https:// 开头".into());
    }
    if url.contains(|c: char| c == ';' || c == '|' || c == '&' || c == '`') {
        return Err("URL 包含非法字符".into());
    }
    Ok(())
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
            name: "豆瓣".to_string(),
            url: "https://pypi.douban.com/simple/".to_string(),
            active: false,
        },
        PipMirror {
            name: "中科大".to_string(),
            url: "https://pypi.mirrors.ustc.edu.cn/simple/".to_string(),
            active: false,
        },
    ]
}

pub async fn list_pip_mirrors(_app_handle: AppHandle) -> Vec<PipMirror> {
    let mut mirrors = get_default_mirrors();
    
    if let Some(active_url) = get_current_mirror().await {
        for mirror in &mut mirrors {
            mirror.active = mirror.url == active_url;
        }
    }
    
    mirrors
}

async fn get_current_mirror() -> Option<String> {
    let result = process_manager::execute_command(
        "pip",
        &["config", "get", "global.index-url"]
    ).await;
    
    match result {
        Ok(output) if output.exit_code == 0 => {
            Some(output.stdout.trim().to_string())
        }
        _ => None
    }
}

pub async fn switch_pip_mirror(app_handle: AppHandle, mirror_name: String, mirror_url: String) -> Result<String, String> {
    if let Err(e) = validate_mirror_url(&mirror_url) {
        return Err(e);
    }
    let result = process_manager::execute_command(
        "pip",
        &["config", "set", "global.index-url", &mirror_url]
    ).await;
    
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
