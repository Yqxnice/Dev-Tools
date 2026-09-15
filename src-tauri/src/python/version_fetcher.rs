use super::super::{download_control, logger, types::AvailablePythonVersion};
use regex::Regex;
use reqwest;
use tauri::AppHandle;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::time::Duration;

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"href="(\d+\.\d+\.\d+)/""#).unwrap());
static STABLE_VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+\.\d+\.\d+$").unwrap());

/// 共享 HTTP 客户端：仅设置连接超时，流式下载不受总超时限制
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

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
    let response = HTTP_CLIENT.get(mirror_url)
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

/// 下载目录：复用公共路径函数，确保与 JetBrains fetcher / lib.rs 命令一致
async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

/// 校验安装包：优先对照镜像侧车 .sha256 文件；无侧车时记录哈希供人工核对
async fn verify_installer(
    app_handle: &AppHandle,
    download_url: &str,
    actual_size: u64,
    declared_size: u64,
    actual_hash: &str,
) -> Result<(), String> {
    // 1. 大小校验（Content-Length 已知时）
    if declared_size > 0 && actual_size != declared_size {
        return Err(format!(
            "文件大小不完整（实际 {} 字节 / 预期 {} 字节），可能下载中断",
            actual_size, declared_size
        ));
    }

    // 2. 尝试获取侧车 .sha256 校验文件（部分镜像提供；python.org 官方通常只有 .asc）
    let sidecar_url = format!("{}.sha256", download_url);
    match HTTP_CLIENT.get(&sidecar_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            let expected = body.split_whitespace().next().unwrap_or("").trim().to_lowercase();
            if expected.len() == 64 && expected == actual_hash {
                logger::info(app_handle, "安装包 SHA256 校验通过（与镜像侧车文件一致）");
            } else if expected.len() == 64 {
                return Err(
                    "SHA256 校验失败：文件哈希与镜像提供的不一致（文件已删除，请重试）".to_string(),
                );
            } else {
                logger::info(
                    app_handle,
                    &format!("安装包 SHA256（侧车文件格式无法解析，仅供核对）: {}", actual_hash),
                );
            }
        }
        _ => {
            logger::info(
                app_handle,
                &format!(
                    "镜像未提供 .sha256 校验文件，安装包 SHA256 为: {}（可到 python.org 人工核对）",
                    actual_hash
                ),
            );
        }
    }

    Ok(())
}

pub async fn download_python(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<PathBuf, String> {
    validate_version_string(&version)?;
    let task_id = format!("python:{}", version);
    let download_url = get_download_url(&version);
    logger::info(&app_handle, &format!("开始下载 Python {}", version));

    let arch = get_system_architecture();
    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(format!("python-{}-{}.exe", version, arch));

    // 下载主流程：暂停/继续/取消/重试由 download_control 统一处理
    let (actual_size, declared_size) = download_control::download_file(
        &app_handle,
        &window,
        &task_id,
        &download_url,
        &file_path,
        &version,
        &HTTP_CLIENT,
        3,
    )
    .await?;

    // 计算 SHA256（非增量式，支持暂停/继续后的完整校验）
    let hash = match download_control::compute_file_hash(&file_path).await {
        Ok(h) => h,
        Err(e) => {
            logger::error(&app_handle, &format!("计算文件哈希失败: {}", e));
            let _ = tokio::fs::remove_file(&file_path).await;
            download_control::emit_failed(&window, &task_id, &version, actual_size, declared_size);
            return Err(e);
        }
    };

    // 校验
    if let Err(verify_err) = verify_installer(
        &app_handle,
        &download_url,
        actual_size,
        declared_size,
        &hash,
    )
    .await
    {
        logger::error(&app_handle, &format!("安装包校验失败: {}", verify_err));
        let _ = tokio::fs::remove_file(&file_path).await;
        download_control::emit_failed(&window, &task_id, &version, actual_size, declared_size);
        return Err(verify_err);
    }

    // 发送完成终态事件
    download_control::emit_completed(&window, &task_id, &version, actual_size, declared_size);
    logger::info(
        &app_handle,
        &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size),
    );
    Ok(file_path)
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
