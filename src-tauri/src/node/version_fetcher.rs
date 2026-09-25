use super::super::{download_control, http_client, logger, types::AvailableNodeVersion};
use regex::Regex;
use std::path::PathBuf;
use tauri::AppHandle;
use std::sync::LazyLock;

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+\.\d+\.\d+$").unwrap());

/// 镜像源列表，按顺序尝试
const MIRRORS: &[(&str, &str)] = &[
    ("阿里云", "https://registry.npmmirror.com/-/binary/node/"),
    ("华为云", "https://mirrors.huaweicloud.com/nodejs/"),
    ("腾讯云", "https://mirrors.cloud.tencent.com/nodejs/"),
    ("官方", "https://nodejs.org/dist/"),
];

/// 从指定镜像获取 index.json
async fn fetch_index(mirror_url: &str) -> Result<Vec<AvailableNodeVersion>, String> {
    let url = format!("{}index.json", mirror_url);
    let response = http_client::default_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let list: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("JSON 解析失败: {}", e))?;

    let mut versions: Vec<AvailableNodeVersion> = list
        .into_iter()
        .filter_map(|item| {
            let version = item.get("version")?.as_str()?;
            // 去掉前导 v
            let version = version.trim_start_matches('v').to_string();
            let is_lts = match item.get("lts") {
                Some(v) => v.is_string(),
                None => false,
            };
            let date = item
                .get("date")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            Some(AvailableNodeVersion {
                version,
                is_lts,
                date,
            })
        })
        .collect();

    // 按版本号倒序（最新在前）
    versions.sort_by(|a, b| {
        let a_parts: Vec<u64> = a.version.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u64> = b.version.split('.').filter_map(|s| s.parse().ok()).collect();
        b_parts.cmp(&a_parts)
    });

    Ok(versions)
}

pub async fn get_available_node_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailableNodeVersion>, String> {
    logger::info(&app_handle, "正在获取可用的 Node.js 版本...");

    for (mirror_name, mirror_url) in MIRRORS {
        match fetch_index(mirror_url).await {
            Ok(mut versions) => {
                // 只保留最近 100 个版本，避免列表过长
                versions.truncate(100);
                logger::info(
                    &app_handle,
                    &format!("从 {} 获取成功，共 {} 个版本", mirror_name, versions.len()),
                );
                return Ok(versions);
            }
            Err(e) => {
                logger::warn(&app_handle, &format!("{} 获取失败: {}", mirror_name, e));
            }
        }
    }

    Err("所有镜像源都无法访问".to_string())
}

pub fn validate_version_string(version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("版本号不能为空".into());
    }
    if !VERSION_REGEX.is_match(version) {
        return Err("版本号格式无效（需形如 20.11.0）".into());
    }
    if version.contains("..") || version.contains('/') || version.contains('\\') {
        return Err("版本号包含非法字符".into());
    }
    Ok(())
}

fn get_system_architecture() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x64"
    }
    #[cfg(target_arch = "x86")]
    {
        "x86"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "arm64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
    {
        "x64"
    }
}

/// 返回 Node.js 指定版本的 .msi 下载链接（默认使用淘宝镜像）
pub fn get_download_url(version: &str) -> String {
    let arch = get_system_architecture();
    format!(
        "https://registry.npmmirror.com/-/binary/node/v{}/node-v{}-{}.msi",
        version, version, arch
    )
}

/// 下载目录：复用公共路径函数
async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

/// 下载 Node.js 安装包并返回文件路径
pub async fn download_node(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<PathBuf, String> {
    validate_version_string(&version)?;
    let task_id = format!("node:{}", version);
    let download_url = get_download_url(&version);
    logger::info(&app_handle, &format!("开始下载 Node.js {}", version));

    let arch = get_system_architecture();
    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(format!("node-v{}-{}.msi", version, arch));

    let (actual_size, declared_size) = download_control::download_file(
        &app_handle,
        &window,
        &task_id,
        &download_url,
        &file_path,
        &version,
        http_client::default_client(),
        3,
    )
    .await?;

    download_control::emit_completed(&window, &task_id, &version, actual_size, declared_size);
    logger::info(
        &app_handle,
        &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size),
    );
    Ok(file_path)
}

/// 只下载 Node.js 安装包，返回路径字符串
pub async fn download_node_only(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<String, String> {
    let installer_path = download_node(app_handle, version, window).await?;
    Ok(installer_path.to_str().unwrap_or("").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_index_sample() {
        // 模拟 nodejs.org/dist/index.json 的单条结构
        let json = r#"[{
          "version": "v20.11.0",
          "date": "2023-11-29",
          "lts": "Iron"
        }, {
          "version": "v21.6.0",
          "date": "2024-01-04",
          "lts": false
        }]"#;
        let list: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let versions: Vec<AvailableNodeVersion> = list
            .into_iter()
            .filter_map(|item| {
                let version = item.get("version")?.as_str()?;
                let version = version.trim_start_matches('v').to_string();
                let is_lts = item.get("lts").map(|v| v.is_string()).unwrap_or(false);
                let date = item.get("date").and_then(|d| d.as_str()).unwrap_or("").to_string();
                Some(AvailableNodeVersion { version, is_lts, date })
            })
            .collect();
        assert_eq!(versions.len(), 2);
        assert!(versions[0].is_lts);
        assert!(!versions[1].is_lts);
    }
}
