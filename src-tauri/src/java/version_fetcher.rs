use super::super::{download_control, http_client, logger, types::AvailableJavaVersion};
use std::sync::LazyLock;
use regex::Regex;
use std::path::PathBuf;
use tauri::AppHandle;

/// 校验 feature version（数字字符串）
static FEATURE_VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+$").unwrap());

/// Adoptium API 基址
const API_BASE: &str = "https://api.adoptium.net";

/// 获取可用版本列表
///
/// 流程：先拉 `/v3/info/available_releases` 得到所有 feature version 和 LTS 列表，
/// 然后对每个 feature version 调 `/v3/assets/feature_releases/{v}/ga` 拉最新 patch 版本。
pub async fn get_available_java_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailableJavaVersion>, String> {
    logger::info(&app_handle, "正在获取可用的 Java 版本...");

    // 第一步：拉 release 列表
    let releases_url = format!("{}/v3/info/available_releases", API_BASE);
    let releases_body = http_client::default_client()
        .get(&releases_url)
        .send()
        .await
        .map_err(|e| format!("请求 available_releases 失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let releases: serde_json::Value = serde_json::from_str(&releases_body)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    let available_releases: Vec<u32> = releases
        .get("available_releases")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect())
        .unwrap_or_default();

    let lts_releases: Vec<u32> = releases
        .get("available_lts_releases")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect())
        .unwrap_or_default();

    if available_releases.is_empty() {
        return Err("Adoptium API 返回空版本列表".to_string());
    }

    // 第二步：对每个 feature version 拉最新 patch 版本
    let mut result: Vec<AvailableJavaVersion> = Vec::new();
    for feature_version in &available_releases {
        match fetch_feature_release_detail(*feature_version).await {
            Ok((most_recent, timestamp)) => {
                let is_lts = lts_releases.contains(feature_version);
                result.push(AvailableJavaVersion {
                    feature_version: *feature_version,
                    is_lts,
                    most_recent_version: most_recent,
                    timestamp,
                });
            }
            Err(e) => {
                logger::warn(
                    &app_handle,
                    &format!("获取 Java {} 详情失败: {}", feature_version, e),
                );
            }
        }
    }

    // 按 feature_version 倒序（最新在前）
    result.sort_by(|a, b| b.feature_version.cmp(&a.feature_version));

    logger::info(
        &app_handle,
        &format!("获取完成，共 {} 个 Java 版本", result.len()),
    );
    Ok(result)
}

/// 获取指定 feature version 的最新 patch 版本和时间戳
async fn fetch_feature_release_detail(feature_version: u32) -> Result<(String, String), String> {
    let url = format!(
        "{}/v3/assets/feature_releases/{}/ga?architecture=x64&heap_size=normal&image_type=jdk&os=windows&page=0&page_size=1&sort_method=DEFAULT&sort_order=DESC&vendor=eclipse",
        API_BASE, feature_version
    );

    let body = http_client::default_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let arr: Vec<serde_json::Value> = serde_json::from_str(&body)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    let first = arr
        .first()
        .ok_or_else(|| format!("Java {} 无可用版本", feature_version))?;

    let version_str = first
        .get("version_data")
        .and_then(|v| v.get("openjdk_version"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let timestamp = first
        .get("timestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    Ok((version_str.to_string(), timestamp.to_string()))
}

/// 校验 feature version 字符串
pub fn validate_feature_version(version: &str) -> Result<u32, String> {
    if version.is_empty() {
        return Err("版本号不能为空".into());
    }
    if !FEATURE_VERSION_REGEX.is_match(version) {
        return Err("版本号格式无效（需为数字，如 17、21）".into());
    }
    version
        .parse::<u32>()
        .map_err(|e| format!("版本号解析失败: {}", e))
}

/// 包类型：installer（.msi 安装包）或 archive（.zip 压缩包）
pub const PACKAGE_TYPE_INSTALLER: &str = "installer";
pub const PACKAGE_TYPE_ARCHIVE: &str = "archive";

/// 根据 package_type 返回对应的文件扩展名
fn package_extension(package_type: &str) -> &'static str {
    match package_type {
        PACKAGE_TYPE_ARCHIVE => "zip",
        _ => "msi",
    }
}

/// 返回指定 feature version 的 JDK 下载链接
///
/// 根据 package_type 选择端点：
/// - `installer` → `/v3/installer/latest/...`（Windows .msi 安装程序）
/// - `archive`   → `/v3/binary/latest/...`（Windows .zip 压缩归档）
pub fn get_download_url(feature_version: u32, package_type: &str) -> String {
    let endpoint = match package_type {
        PACKAGE_TYPE_ARCHIVE => "binary",
        _ => "installer",
    };
    format!(
        "{}/v3/{}/latest/{}/ga/windows/x64/jdk/hotspot/normal/eclipse",
        API_BASE, endpoint, feature_version
    )
}

/// 下载目录：复用公共路径函数
async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

/// 下载 Java (JDK) 安装包/压缩包并返回文件路径
pub async fn download_java(
    app_handle: AppHandle,
    feature_version: u32,
    package_type: &str,
    window: tauri::Window,
) -> Result<PathBuf, String> {
    let task_id = format!("java:{}:{}", feature_version, package_type);
    let download_url = get_download_url(feature_version, package_type);
    let ext = package_extension(package_type);
    logger::info(
        &app_handle,
        &format!("开始下载 Java {} ({})", feature_version, package_type),
    );

    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(format!(
        "openjdk-{}-windows-x64-bin.{}",
        feature_version, ext
    ));

    let (actual_size, declared_size) = download_control::download_file(
        &app_handle,
        &window,
        &task_id,
        &download_url,
        &file_path,
        &feature_version.to_string(),
        http_client::default_client(),
        3,
    )
    .await?;

    download_control::emit_completed(
        &window,
        &task_id,
        &feature_version.to_string(),
        actual_size,
        declared_size,
    );
    logger::info(
        &app_handle,
        &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size),
    );
    Ok(file_path)
}

/// 只下载 Java 安装包/压缩包，返回路径字符串
pub async fn download_java_only(
    app_handle: AppHandle,
    feature_version: u32,
    package_type: &str,
    window: tauri::Window,
) -> Result<String, String> {
    let installer_path = download_java(app_handle, feature_version, package_type, window).await?;
    Ok(installer_path.to_str().unwrap_or("").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_feature_version_accepts_numeric() {
        assert_eq!(validate_feature_version("17").unwrap(), 17);
        assert_eq!(validate_feature_version("21").unwrap(), 21);
    }

    #[test]
    fn validate_feature_version_rejects_empty() {
        assert!(validate_feature_version("").is_err());
    }

    #[test]
    fn validate_feature_version_rejects_non_numeric() {
        assert!(validate_feature_version("17.0").is_err());
        assert!(validate_feature_version("abc").is_err());
    }

    #[test]
    fn download_url_installer_uses_installer_endpoint() {
        let url = get_download_url(17, PACKAGE_TYPE_INSTALLER);
        assert!(url.contains("/latest/17/"));
        assert!(url.contains("adoptium.net"));
        assert!(url.contains("/v3/installer/"));
    }

    #[test]
    fn download_url_archive_uses_binary_endpoint() {
        let url = get_download_url(21, PACKAGE_TYPE_ARCHIVE);
        assert!(url.contains("/latest/21/"));
        assert!(url.contains("/v3/binary/"));
    }

    #[test]
    fn package_extension_maps_correctly() {
        assert_eq!(package_extension(PACKAGE_TYPE_INSTALLER), "msi");
        assert_eq!(package_extension(PACKAGE_TYPE_ARCHIVE), "zip");
        assert_eq!(package_extension("unknown"), "msi"); // 默认回退到 msi
    }
}
