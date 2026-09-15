use super::super::{logger, types::AvailableJavaVersion};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest;
use tauri::AppHandle;
use std::time::Duration;

/// 校验 feature version（数字字符串）
static FEATURE_VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+$").unwrap());

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

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
    let releases_body = HTTP_CLIENT
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

    let body = HTTP_CLIENT
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

/// 返回指定 feature version 的 JDK 下载链接（.msi 安装包）
///
/// 使用 Adoptium 的 latest 重定向链接，自动指向最新 patch 版本：
/// https://api.adoptium.net/v3/binary/latest/{version}/ga/windows/x64/jdk/hotspot/normal/eclipse
pub fn get_download_url(feature_version: u32) -> String {
    format!(
        "{}/v3/binary/latest/{}/ga/windows/x64/jdk/hotspot/normal/eclipse",
        API_BASE, feature_version
    )
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
    fn download_url_contains_feature_version() {
        let url = get_download_url(17);
        assert!(url.contains("/latest/17/"));
        assert!(url.contains("adoptium.net"));
    }
}
