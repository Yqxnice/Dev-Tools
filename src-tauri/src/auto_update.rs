//! 应用自动更新检查（Feature 18）
//!
//! 设计原则（MVP）：
//! - 仅"检查 + 提示"，不自动下载安装（避免引入 signtool + updater 密钥的发布基础设施）
//! - 复用 http_client 的 reqwest 客户端，不新增依赖
//! - 失败时静默降级：网络错误返回 Err，前端可吞掉；404（无 release）返回 has_update=false
//!
//! 用户在弹窗中点击"立即查看"会通过 @tauri-apps/plugin-shell 打开 release 页面。

use crate::http_client;
use crate::types::UpdateInfo;
use serde::Deserialize;

/// GitHub Releases API 端点：返回最新发布
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/Yqxnice/Dev-Tools/releases/latest";

/// GitHub Release API 响应（仅取需要的字段）
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    /// 发布标签，形如 "v0.2.0"
    tag_name: String,
    /// 发布标题（与 tag_name 可能相同）
    #[allow(dead_code)]
    name: Option<String>,
    /// 发布说明（changelog），可能为空
    body: Option<String>,
    /// 浏览器可访问的 URL
    html_url: String,
    /// 发布时间（ISO 8601）
    published_at: String,
}

/// 比较版本号：返回 true 表示 latest > current
///
/// 输入格式形如 "0.1.0" 或 "v0.2.0"；前导 `v` 会自动去除。
/// 比较规则：按 `.` 分段为数字数组，逐段比较；缺失段视为 0。
/// 任一版本无法解析时返回 false（保守降级）。
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let cur = parse_version(current);
    let lat = parse_version(latest);
    if cur.is_empty() || lat.is_empty() {
        return false;
    }
    let n = cur.len().max(lat.len());
    for i in 0..n {
        let c = cur.get(i).copied().unwrap_or(0);
        let l = lat.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// 把版本字符串切分为数字数组；前导 `v` 自动去除；非数字段丢弃
fn parse_version(s: &str) -> Vec<u64> {
    s.trim()
        .trim_start_matches('v')
        .trim()
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect()
}

/// 检查应用更新：调用 GitHub Releases API，对比当前版本与最新发布版本。
///
/// - 网络失败 → Err（前端可静默吞掉或提示用户）
/// - HTTP 404（仓库尚无 release）→ Ok(UpdateInfo { has_update: false, ... })
/// - HTTP 200 → 解析 tag_name，与当前版本比较
#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();

    let resp = http_client::default_client()
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", "Dev-Tools-Updater")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("GitHub API 请求失败: {}", e))?;

    // 404 表示仓库尚无 release：不算错误，返回 has_update=false
    if resp.status().as_u16() == 404 {
        return Ok(UpdateInfo {
            has_update: false,
            current_version,
            latest_version: String::new(),
            html_url: String::new(),
            body: None,
            published_at: None,
        });
    }

    if !resp.status().is_success() {
        return Err(format!(
            "GitHub API 返回状态码: {}",
            resp.status().as_u16()
        ));
    }

    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| format!("解析 GitHub 响应失败: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let has_update = is_newer_version(&current_version, &latest_version);

    Ok(UpdateInfo {
        has_update,
        current_version,
        latest_version,
        html_url: release.html_url,
        body: release.body,
        published_at: Some(release.published_at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.0", "1.0.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.2.0", "0.1.0"));
    }

    #[test]
    fn test_is_newer_with_v_prefix() {
        assert!(is_newer_version("0.1.0", "v0.2.0"));
        assert!(is_newer_version("v0.1.0", "v0.2.0"));
        assert!(!is_newer_version("v0.1.0", "v0.1.0"));
    }

    #[test]
    fn test_is_newer_different_length() {
        assert!(is_newer_version("0.1", "0.1.1"));
        assert!(is_newer_version("1.2.3", "1.2.3.1"));
        assert!(!is_newer_version("1.2", "1.2.0"));
    }

    #[test]
    fn test_is_newer_invalid_input() {
        assert!(!is_newer_version("0.1.0", "abc"));
        assert!(!is_newer_version("abc", "0.1.0"));
    }

    #[test]
    fn test_is_newer_higher_major() {
        assert!(is_newer_version("0.9.9", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "0.9.9"));
    }

    #[test]
    fn test_is_newer_same_version_different_segments() {
        assert!(!is_newer_version("1.2.3", "1.2.3"));
        assert!(is_newer_version("1.2.3", "1.2.4"));
        assert!(!is_newer_version("1.2.4", "1.2.3"));
    }
}
