use super::super::{logger, types::{AvailablePythonVersion, DownloadProgress}};
use regex::Regex;
use reqwest;
use tauri::{AppHandle, Emitter};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::time::Duration;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"href="(\d+\.\d+\.\d+)/""#).unwrap());
static STABLE_VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+\.\d+\.\d+$").unwrap());

const MIRRORS: &[(&str, &str)] = &[
    ("华为云", "https://mirrors.huaweicloud.com/python/"),
    ("淘宝", "https://registry.npmmirror.com/binary.html?path=python/"),
    ("腾讯云", "https://mirrors.cloud.tencent.com/python/"),
    ("官方", "https://www.python.org/ftp/python/"),
];

pub fn validate_version_string(version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("版本号不能为空".into());
    }
    if !STABLE_VERSION_REGEX.is_match(version) {
        return Err("版本号格式无效".into());
    }
    if version.contains("..") || version.contains('/') || version.contains('\\') {
        return Err("版本号包含非法字符".into());
    }
    Ok(())
}

async fn fetch_versions_from_mirror(mirror_url: &str) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建请求客户端失败: {}", e))?;
    let response = client.get(mirror_url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let mut versions: Vec<String> = VERSION_REGEX
        .captures_iter(&html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    versions.sort_by(|a, b| {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
        b_parts.cmp(&a_parts)
    });

    Ok(versions)
}

pub async fn get_available_python_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailablePythonVersion>, String> {
    logger::info(&app_handle, "正在获取可用的 Python 版本...");

    for (_mirror_name, mirror_url) in MIRRORS {
        match fetch_versions_from_mirror(mirror_url).await {
            Ok(versions) => {
                let available_versions: Vec<AvailablePythonVersion> = versions
                    .into_iter()
                    .take(50)
                    .map(|version| AvailablePythonVersion {
                        is_stable: STABLE_VERSION_REGEX.is_match(&version),
                        version,
                        release_date: None,
                        download_urls: vec![
                            format!("{}windows/", mirror_url),
                        ],
                    })
                    .collect();

                logger::info(
                    &app_handle,
                    &format!("获取成功，共 {} 个版本", available_versions.len()),
                );
                return Ok(available_versions);
            }
            Err(e) => {
                logger::warn(
                    &app_handle,
                    &format!("获取失败: {}", e),
                );
            }
        }
    }

    Err("所有镜像源都无法访问".to_string())
}

fn get_system_architecture() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "amd64"
    }
    #[cfg(target_arch = "x86")]
    {
        "win32"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "arm64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
    {
        "amd64" // 默认为 amd64
    }
}

pub fn get_download_url(version: &str) -> String {
    // 默认使用华为云镜像
    let arch = get_system_architecture();
    format!(
        "https://mirrors.huaweicloud.com/python/{}/python-{}-{}.exe",
        version, version, arch
    )
}

async fn get_download_dir() -> PathBuf {
    if let Some(dir) = dirs::desktop_dir() {
        let _ = tokio::fs::create_dir_all(&dir).await;
        dir
    } else if let Some(mut dir) = dirs::download_dir() {
        dir.push("PythonInstallers");
        let _ = tokio::fs::create_dir_all(&dir).await;
        dir
    } else {
        PathBuf::from(".")
    }
}

pub async fn download_python(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<PathBuf, String> {
    validate_version_string(&version)?;
    let download_url = get_download_url(&version);
    logger::info(&app_handle, &format!("开始下载 Python {}", version));
    
    let arch = get_system_architecture();
    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(format!("python-{}-{}.exe", version, arch));
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建请求客户端失败: {}", e))?;
    let response = client.get(&download_url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status()));
    }
    
    let total_size = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;
    
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载数据失败: {}", e))?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|e| format!("写入文件失败: {}", e))?;
        
        let percentage = if total_size > 0 {
            (downloaded as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };
        
        let progress = DownloadProgress {
            version: version.clone(),
            downloaded,
            total: total_size,
            percentage,
            status: "下载中".to_string(),
            completed: false,
            success: false,
        };
        
        window.emit("download_progress", &progress)
            .map_err(|e| format!("发送进度事件失败: {}", e))?;
    }
    
    // 发送完成事件
    let final_progress = DownloadProgress {
        version: version.clone(),
        downloaded: total_size,
        total: total_size,
        percentage: 100.0,
        status: "下载完成".to_string(),
        completed: true,
        success: true,
    };
    window.emit("download_progress", &final_progress)
        .map_err(|e| format!("发送下载完成事件失败: {}", e))?;
    
    logger::info(&app_handle, "下载完成");
    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_version_string_rejects_empty() {
        assert!(validate_version_string("").is_err());
    }

    #[test]
    fn validate_version_string_rejects_invalid_format() {
        assert!(validate_version_string("3").is_err());
        assert!(validate_version_string("3.12").is_err());
        assert!(validate_version_string("abc").is_err());
    }

    #[test]
    fn validate_version_string_rejects_path_traversal() {
        assert!(validate_version_string("3.12.0/../").is_err());
        assert!(validate_version_string("3.12.0\\windows").is_err());
    }

    #[test]
    fn validate_version_string_accepts_valid() {
        assert!(validate_version_string("3.12.0").is_ok());
        assert!(validate_version_string("3.11.5").is_ok());
        assert!(validate_version_string("3.10.11").is_ok());
    }

    #[test]
    fn get_download_url_uses_huawei_mirror() {
        let url = get_download_url("3.12.0");
        assert!(url.contains("mirrors.huaweicloud.com"));
        assert!(url.contains("python-3.12.0-"));
        assert!(url.ends_with(".exe"));
    }

    #[test]
    fn get_system_architecture_returns_non_empty() {
        let arch = get_system_architecture();
        assert!(!arch.is_empty());
        assert!(arch == "amd64" || arch == "win32" || arch == "arm64");
    }

    #[test]
    fn stable_version_regex_matches_valid() {
        assert!(STABLE_VERSION_REGEX.is_match("3.12.0"));
        assert!(STABLE_VERSION_REGEX.is_match("3.11.5"));
        assert!(STABLE_VERSION_REGEX.is_match("2.7.18"));
        assert!(!STABLE_VERSION_REGEX.is_match("3.12.0rc1"));
        assert!(!STABLE_VERSION_REGEX.is_match("3.12"));
    }
}

// 只下载 Python 安装包
pub async fn download_python_only(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<String, String> {
    let installer_path = download_python(app_handle, version, window).await?;
    Ok(installer_path.to_str().unwrap_or("").to_string())
}
