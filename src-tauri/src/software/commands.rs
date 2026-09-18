//! 软件图标兜底解析：第三方 favicon 服务在前端加载失败时，由后端抓取官网
//! HTML，解析 <link rel="icon"> / apple-touch-icon / manifest / og:image，
//! 下载图标字节并缓存到本地，返回 data URI 供前端 <img> 直接渲染。
//!
//! 必须在后端做：webview 内 fetch 跨站 HTML 会被 CORS 拦截。

use base64::Engine;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::PathBuf;

use crate::http_client;

static LINK_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<link\b[^>]*>").unwrap());
static META_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<meta\b[^>]*>").unwrap());

/// 图标缓存目录：{系统缓存目录}/DevTools/software-icons
fn icon_cache_dir() -> Result<PathBuf, String> {
    let base = dirs::cache_dir()
        .ok_or_else(|| "无法定位系统缓存目录".to_string())?
        .join("DevTools")
        .join("software-icons");
    std::fs::create_dir_all(&base).map_err(|e| format!("创建图标缓存目录失败: {}", e))?;
    Ok(base)
}

/// 从标签中提取属性值（兼容单/双引号）
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pat = format!(r#"(?i)\b{}\s*=\s*("([^"]*)"|'([^']*)')"#, attr);
    let re = Regex::new(&pat).ok()?;
    let caps = re.captures(tag)?;
    caps.get(2).or_else(|| caps.get(3)).map(|m| m.as_str().to_string())
}

/// rel 是否包含指定关键词（兼容 "icon alternate" 等多值写法）
fn rel_contains(rel: &str, keyword: &str) -> bool {
    rel.split_whitespace()
        .any(|v| v.eq_ignore_ascii_case(keyword))
}

/// 根据扩展名推断 MIME
fn mime_from_ext(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        // "icon" 兼容历史缓存文件名（*.vnd.microsoft.icon 的 extension 为 "icon"）
        "ico" | "icon" | "x-icon" => "image/x-icon",
        _ => "image/x-icon",
    }
}

/// 从 URL 路径提取图片扩展名，无法识别返回 ico
fn ext_from_url(url: &reqwest::Url) -> String {
    let path = url.path().to_ascii_lowercase();
    for e in ["svg", "png", "webp", "jpg", "jpeg", "gif", "ico"] {
        if path.ends_with(&format!(".{}", e)) {
            return e.to_string();
        }
    }
    "ico".to_string()
}

/// 域名转安全文件名
fn host_to_filename(host: &str) -> String {
    host.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// 净化文件扩展名：只保留字母、数字、`+`、`-`，防止响应头注入路径穿越字符
fn sanitize_ext(ext: &str) -> String {
    ext.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '-')
        .collect()
}

/// 候选图标：(优先级, URL)。优先级 0 最高
type Candidate = (u8, reqwest::Url);

/// 纯同步解析 HTML，收集带优先级的候选图标
/// - 0: <link rel="icon"> 中的非 ico（svg/png 通常更清晰）
/// - 1: <link rel="icon"> 中的 ico
/// - 2: apple-touch-icon
/// - 3: web manifest（需二次请求展开）
/// - 4: og:image
/// - 9: 根目录 /favicon.ico
fn parse_html_candidates(html: &str, base: &reqwest::Url) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();

    for tag in LINK_TAG_RE.find_iter(html).map(|m| m.as_str()) {
        let rel = extract_attr(tag, "rel").unwrap_or_default();
        let href = match extract_attr(tag, "href") {
            Some(h) if !h.is_empty() => h,
            _ => continue,
        };
        let resolved = match base.join(&href) {
            Ok(u) => u,
            Err(_) => continue,
        };
        if rel_contains(&rel, "apple-touch-icon") {
            candidates.push((2, resolved));
        } else if rel_contains(&rel, "icon") {
            let priority = if resolved.path().to_ascii_lowercase().ends_with(".ico") { 1 } else { 0 };
            candidates.push((priority, resolved));
        } else if rel_contains(&rel, "manifest") {
            candidates.push((3, resolved));
        }
    }

    for tag in META_TAG_RE.find_iter(html).map(|m| m.as_str()) {
        let prop = extract_attr(tag, "property").unwrap_or_default();
        if prop.eq_ignore_ascii_case("og:image") {
            if let Some(content) = extract_attr(tag, "content") {
                if let Ok(u) = base.join(&content) {
                    candidates.push((4, u));
                }
            }
        }
    }

    if let Ok(fallback) = base.join("/favicon.ico") {
        candidates.push((9, fallback));
    }

    candidates
}

/// 请求并解析 web manifest，返回其中 sizes 最大的图标 URL
async fn resolve_manifest(
    client: &reqwest::Client,
    manifest_url: &reqwest::Url,
) -> Option<reqwest::Url> {
    let json: serde_json::Value = client
        .get(manifest_url.clone())
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let icons = json.get("icons")?.as_array()?;
    let mut best: Option<(u32, reqwest::Url)> = None;
    for icon in icons {
        let src = icon.get("src")?.as_str()?;
        let url = manifest_url.join(src).ok()?;
        let size_score = icon
            .get("sizes")
            .and_then(|v| v.as_str())
            .and_then(|s| s.split('x').next())
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0);
        if best.as_ref().map(|(s, _)| size_score >= *s).unwrap_or(true) {
            best = Some((size_score, url));
        }
    }
    best.map(|(_, u)| u)
}

/// 下载候选图标，返回 (mime, bytes)
async fn try_download_icon(
    client: &reqwest::Client,
    url: &reqwest::Url,
) -> Result<(String, Vec<u8>), String> {
    let resp = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| format!("下载图标失败 {}: {}", url, e))?;
    if !resp.status().is_success() {
        return Err(format!("图标返回状态码 {}: {}", resp.status(), url));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取图标字节失败: {}", e))?
        .to_vec();
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err(format!("图标大小异常 ({} 字节)", bytes.len()));
    }
    let mime = if content_type.starts_with("image/") {
        content_type.split(';').next().unwrap_or("image/png").to_string()
    } else {
        mime_from_ext(&ext_from_url(url)).to_string()
    };
    Ok((mime, bytes))
}

/// 解析官网图标：优先读本地缓存，否则抓取 HTML 解析 <link> 后下载缓存。
/// 成功返回 `data:{mime};base64,...`，全部失败返回 None。
#[tauri::command]
pub async fn resolve_software_icon(page_url: String) -> Result<Option<String>, String> {
    let parsed = reqwest::Url::parse(&page_url).map_err(|e| format!("非法 URL: {}", e))?;
    let host = parsed.host_str().unwrap_or("unknown").to_string();
    let cache_dir = icon_cache_dir()?;
    let file_stem = host_to_filename(&host);

    // 1. 命中本地缓存：{stem}.{ext}，扩展名携带 mime 信息
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_stem().and_then(|s| s.to_str()) == Some(file_stem.as_str()) {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let data_uri = format!(
                            "data:{};base64,{}",
                            mime_from_ext(ext),
                            base64::engine::general_purpose::STANDARD.encode(&bytes)
                        );
                        return Ok(Some(data_uri));
                    }
                }
            }
        }
    }

    // 2. 抓取官网首页（best-effort）。
    //    部分站点（如 mysql.com 走 Akamai WAF）会对非浏览器 TLS 指纹返回 403，
    //    但其静态资源 /favicon.ico 仍可直接下载——因此首页失败不能直接放弃，
    //    退化为"无 HTML 候选，仅尝试根目录 favicon.ico"。
    let client = http_client::browser_client();
    let mut candidates: Vec<Candidate> = Vec::new();
    let base_url = match client.get(&page_url).send().await {
        Ok(page_resp) => {
            let final_url = page_resp.url().clone();
            if page_resp.status().is_success() {
                if let Ok(html) = page_resp.text().await {
                    candidates = parse_html_candidates(&html, &final_url);
                }
            }
            final_url
        }
        Err(_) => parsed.clone(),
    };

    // 无论首页解析成功与否，根目录 favicon.ico 始终作为兜底候选（去重）
    if let Ok(fallback) = base_url.join("/favicon.ico") {
        if !candidates.iter().any(|(_, u)| *u == fallback) {
            candidates.push((9, fallback));
        }
    }

    // 展开 manifest（其图标优先级最高）
    let manifest_urls: Vec<reqwest::Url> = {
        let mut urls = Vec::new();
        for (priority, url) in &candidates {
            if *priority == 3 {
                if let Some(icon_url) = resolve_manifest(&client, url).await {
                    urls.push(icon_url);
                }
            }
        }
        urls
    };

    candidates.sort_by_key(|(p, _)| *p);
    let mut ordered: Vec<reqwest::Url> = manifest_urls;
    for (_, url) in candidates {
        if !ordered.contains(&url) {
            ordered.push(url);
        }
    }

    // 3. 依次尝试候选 URL
    for icon_url in ordered {
        if let Ok((mime, bytes)) = try_download_icon(&client, &icon_url).await {
            let ext = match mime.rsplit('/').next().unwrap_or("ico") {
                "svg+xml" => "svg".to_string(),
                "jpeg" => "jpg".to_string(),
                // image/x-icon、image/vnd.microsoft.icon 统一规范为 .ico
                "x-icon" | "vnd.microsoft.icon" => "ico".to_string(),
                other => sanitize_ext(other),
            };
            // 写缓存（best-effort，失败不影响返回）
            let cache_path = cache_dir.join(format!("{}.{}", file_stem, ext));
            let _ = std::fs::write(&cache_path, &bytes);
            let data_uri = format!(
                "data:{};base64,{}",
                mime,
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            );
            return Ok(Some(data_uri));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_relative_icon_url_is_resolved() {
        // codebuddy.cn 真实案例：href 为 //cdn/... 协议相对 URL
        let html = r#"<html><head>
        <link rel="icon" href="//download.codebuddy.cn/web/website/assets/logo.svg" />
        </head></html>"#;
        let base = reqwest::Url::parse("https://www.codebuddy.cn/").unwrap();
        let candidates = parse_html_candidates(html, &base);
        assert_eq!(
            candidates[0].1.as_str(),
            "https://download.codebuddy.cn/web/website/assets/logo.svg"
        );
        assert_eq!(candidates[0].0, 0); // svg 非 ico，最高优先级
    }

    #[test]
    fn icon_priority_order() {
        let html = r#"
        <link rel="manifest" href="/manifest.json">
        <link rel="apple-touch-icon" href="/apple.png">
        <link rel="icon" href="/favicon.ico">
        <link rel="icon" type="image/svg+xml" href="/logo.svg">
        <meta property="og:image" content="https://cdn.example/og.png">
        "#;
        let base = reqwest::Url::parse("https://example.com/").unwrap();
        let candidates = parse_html_candidates(html, &base);
        let urls: Vec<(u8, &str)> =
            candidates.iter().map(|(p, u)| (*p, u.path())).collect();
        assert!(urls.contains(&(0, "/logo.svg")));
        assert!(urls.contains(&(1, "/favicon.ico")));
        assert!(urls.contains(&(2, "/apple.png")));
        assert!(urls.contains(&(3, "/manifest.json")));
        assert!(urls.contains(&(4, "/og.png")));
        assert!(urls.contains(&(9, "/favicon.ico"))); // 根目录兜底
    }

    #[test]
    fn relative_and_absolute_hrefs() {
        let html = r#"
        <link rel="icon" href="assets/icon.png">
        <link rel="shortcut icon" href="https://cdn.other.test/x.ico">
        "#;
        let base = reqwest::Url::parse("https://a.test/sub/page.html").unwrap();
        let candidates = parse_html_candidates(html, &base);
        let paths: Vec<String> = candidates.iter().map(|(_, u)| u.as_str().to_string()).collect();
        // 相对路径相对最终页面 URL 的目录解析
        assert!(paths.contains(&"https://a.test/sub/assets/icon.png".to_string()));
        assert!(paths.contains(&"https://cdn.other.test/x.ico".to_string()));
    }

    #[test]
    fn extract_attr_handles_both_quotes() {
        assert_eq!(
            extract_attr(r#"<link rel='icon' href='/a.png'>"#, "href"),
            Some("/a.png".to_string())
        );
        assert_eq!(
            extract_attr(r#"<LINK REL="shortcut icon" HREF="/b.ico">"#, "rel"),
            Some("shortcut icon".to_string())
        );
        assert_eq!(extract_attr(r#"<link rel="icon">"#, "href"), None);
    }

    /// 真实网络链路验证（需联网，默认跳过）：
    /// mysql.com 首页被 Akamai WAF 返回 403，但 /favicon.ico 可直接下载，
    /// 验证"首页失败后退化为直连 favicon"的兜底链路端到端可用。
    /// 运行：cargo test software -- --ignored
    #[tokio::test]
    #[ignore]
    async fn mysql_waf_blocked_home_but_favicon_downloadable() {
        let result = super::resolve_software_icon("https://www.mysql.com/".to_string())
            .await
            .expect("命令不应返回 Err");
        let uri = result.expect("应通过 /favicon.ico 兜底拿到图标");
        assert!(uri.starts_with("data:image/"), "返回应为 data URI，实际前缀: {}", &uri[..uri.len().min(30)]);
    }
}
