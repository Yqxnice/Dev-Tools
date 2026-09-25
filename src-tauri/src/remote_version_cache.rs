//! 远程版本数据缓存：先尝试从远程 URL 拉取最新版本数据，
//! 缓存到本地文件系统，拉取失败时回退到嵌入的编译期数据。
//!
//! 缓存策略：
//! - 缓存文件位于 `{缓存目录}/DevTools/version-cache/{key}.json`
//! - 缓存有效期 24 小时，过期后重新拉取
//! - 拉取失败时静默回退到嵌入数据，不影响用户体验

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 缓存有效期：24 小时（秒）
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// 缓存目录：{系统缓存目录}/DevTools/version-cache
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DevTools")
        .join("version-cache")
}

/// 缓存文件路径
fn cache_path(key: &str) -> PathBuf {
    cache_dir().join(format!("{}.json", key))
}

/// 缓存元数据（时间戳）
fn meta_path(key: &str) -> PathBuf {
    cache_dir().join(format!("{}.meta", key))
}

/// 读取缓存的 JSON 字符串，若不存在或过期则返回 None
fn read_cache(key: &str) -> Option<String> {
    let meta = meta_path(key);
    let data = cache_path(key);

    if !meta.exists() || !data.exists() {
        return None;
    }

    // 读取时间戳
    let ts_str = std::fs::read_to_string(&meta).ok()?;
    let ts: u64 = ts_str.trim().parse().ok()?;

    // 检查是否过期
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if now.saturating_sub(ts) > CACHE_TTL_SECS {
        return None;
    }

    std::fs::read_to_string(data).ok()
}

/// 写入缓存
fn write_cache(key: &str, data: &str) {
    let dir = cache_dir();
    let _ = std::fs::create_dir_all(&dir);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let _ = std::fs::write(meta_path(key), now.to_string());
    let _ = std::fs::write(cache_path(key), data);
}

/// 从远程 URL 拉取版本数据，带回缓存和嵌入数据回退。
///
/// - `key`：缓存键名（如 "mysql_versions"、"postgresql_versions"）
/// - `remote_url`：远程 JSON 数据的 URL
/// - `embedded`：编译期嵌入的 JSON 字符串（兜底）
/// - `app_handle`：用于日志输出
///
/// 返回最新的 JSON 字符串（可能是远程的、缓存的、或嵌入的）
pub async fn fetch_version_data(
    key: &str,
    remote_url: &str,
    embedded: &str,
    app_handle: &tauri::AppHandle,
) -> String {
    // 1. 尝试读取本地缓存
    if let Some(cached) = read_cache(key) {
        crate::logger::info(
            app_handle,
            &format!("使用缓存的 {} 版本数据", key),
        );
        return cached;
    }

    // 2. 尝试从远程拉取
    match crate::http_client::default_client()
        .get(remote_url)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            match resp.text().await {
                Ok(body) if !body.is_empty() => {
                    // 验证 JSON 可解析
                    if serde_json::from_str::<serde_json::Value>(&body).is_ok() {
                        write_cache(key, &body);
                        crate::logger::info(
                            app_handle,
                            &format!("从远程更新 {} 版本数据成功", key),
                        );
                        return body;
                    }
                    crate::logger::warn(
                        app_handle,
                        &format!("远程 {} 数据 JSON 解析失败，使用嵌入数据", key),
                    );
                }
                Err(e) => {
                    crate::logger::warn(
                        app_handle,
                        &format!("读取远程 {} 响应失败: {}，使用嵌入数据", key, e),
                    );
                }
                _ => {}
            }
        }
        Err(e) => {
            crate::logger::warn(
                app_handle,
                &format!("远程 {} 数据拉取失败: {}，使用嵌入数据", key, e),
            );
        }
        _ => {
            crate::logger::warn(
                app_handle,
                &format!("远程 {} 数据返回非 200，使用嵌入数据", key),
            );
        }
    }

    // 3. 回退到嵌入数据
    crate::logger::info(app_handle, &format!("使用嵌入的 {} 版本数据", key));
    embedded.to_string()
}
