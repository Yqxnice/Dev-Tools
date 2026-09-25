//! 软件图标兜底解析：第三方 favicon 服务在前端加载失败时，由后端抓取官网
//! HTML，解析 <link rel="icon"> / apple-touch-icon / manifest / og:image，
//! 下载图标字节并缓存到本地，返回 data URI 供前端 <img> 直接渲染。
//!
//! 必须在后端做：webview 内 fetch 跨站 HTML 会被 CORS 拦截。

use base64::Engine;
use std::sync::LazyLock;
use regex::Regex;
use std::net::IpAddr;
use std::path::PathBuf;

use crate::http_client;
use crate::error::AppError;

/// HTML 响应体最大字节数（1MB），防止恶意站点返回超大内容导致内存耗尽
const MAX_HTML_BODY_BYTES: usize = 1 * 1024 * 1024;

/// 检查 URL 是否指向内网/保留地址（SSRF 防护）
fn is_private_or_reserved_ip(url: &reqwest::Url) -> bool {
    let host = match url.host_str() {
        Some(h) => h,
        None => return true,
    };
    // 解析为 IP 地址
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => {
                v4.is_loopback()            // 127.x.x.x
                    || v4.is_link_local()    // 169.254.x.x
                    || v4.is_private()       // 10.x.x.x, 172.16-31.x.x, 192.168.x.x
                    || v4.is_broadcast()
                    || v4.is_unspecified()   // 0.0.0.0
                    || v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64 // 100.64-127.x.x (CGNAT)
                    || v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0 // 192.0.0.x
                    || v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 2 // 192.0.2.x (documentation)
                    || v4.octets()[0] == 198 && v4.octets()[1] == 18 && v4.octets()[2] == 0 // 198.18.x.x (benchmarking)
                    || v4.octets()[0] == 198 && v4.octets()[1] == 51 && v4.octets()[2] == 100 // 198.51.100.x (documentation)
                    || v4.octets()[0] == 203 && v4.octets()[1] == 0 && v4.octets()[2] == 113 // 203.0.113.x (documentation)
                    || v4.octets()[0] >= 224 // multicast + reserved
            }
            IpAddr::V6(v6) => {
                v6.is_loopback()            // ::1
                    || v6.is_unspecified()   // ::
                    || v6.is_unicast_link_local() // fe80::/10
                    || v6.octets()[0] == 0xff // multicast
            }
        };
    }
    // 非 IP 主名：检查 localhost 等常见内网域名
    let lower = host.to_ascii_lowercase();
    lower == "localhost"
        || lower.ends_with(".localhost")
        || lower == "0.0.0.0"
        || lower == "127.0.0.1"
        || lower == "::1"
}

static LINK_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>").unwrap());
static META_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<meta\b[^>]*>").unwrap());

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
    // SSRF 阻止 manifest 指向内网地址
    if is_private_or_reserved_ip(manifest_url) {
        return None;
    }
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
    // SSRF 阻止下载内网地址的资源
    if is_private_or_reserved_ip(url) {
        return Err(format!("不允许访问内网地址: {}", url));
    }
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

    // SSRF 防护：阻止访问内网/保留地址
    if is_private_or_reserved_ip(&parsed) {
        return Err("不允许访问内网或保留地址".to_string());
    }

    // 仅允许 http/https 协议
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("仅支持 http/https 协议".to_string());
    }

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

    let client = http_client::browser_client();

    // 2. 先直接尝试 favicon.ico（快速路径：几 KB 文件，不需要下载整个页面）
    //    大部分站点都有 /favicon.ico，成功即可直接返回，避免下载 1-2MB HTML。
    if let Ok(favicon_url) = parsed.join("/favicon.ico") {
        if let Ok((mime, bytes)) = try_download_icon(&client, &favicon_url).await {
            let ext = match mime.rsplit('/').next().unwrap_or("ico") {
                "svg+xml" => "svg".to_string(),
                "jpeg" => "jpg".to_string(),
                "x-icon" | "vnd.microsoft.icon" => "ico".to_string(),
                other => sanitize_ext(other),
            };
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

    // 3. favicon.ico 失败：下载 HTML 页面解析 <link> 候选（慢路径）
    //    超时 / body 过大 / 读取失败均不报错，降级为空候选列表继续。
    let mut candidates: Vec<Candidate> = Vec::new();
    let base_url = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.get(&page_url).send(),
    ).await {
        Ok(Ok(page_resp)) => {
            let final_url = page_resp.url().clone();
            if page_resp.status().is_success() {
                // 限制 HTML 响应体大小，防止恶意站点返回超大内容导致内存耗尽
                match page_resp.bytes().await {
                    Ok(bytes) => {
                        if bytes.len() <= MAX_HTML_BODY_BYTES {
                            if let Ok(html) = String::from_utf8(bytes.to_vec()) {
                                candidates = parse_html_candidates(&html, &final_url);
                            }
                        }
                        // 超大 body：静默跳过，降级到候选 favicon.ico
                    }
                    Err(_) => { /* body 读取失败：静默跳过 */ }
                }
            }
            final_url
        }
        _ => parsed.clone(), // 超时或发送失败：用原始 URL 继续
    };

    // 根目录 favicon.ico 始终作为兜底候选（去重）
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

// ──────────────────────────────────────────────────────────────────
// 软件安装状态检测（Feature 7）
// ──────────────────────────────────────────────────────────────────

/// 软件安装检测结果
#[derive(serde::Serialize, Clone, Debug)]
pub struct InstalledStatus {
    /// 是否已安装（PATH 中能找到且 `--version` 执行成功）
    pub installed: bool,
    /// 版本号（命令 stdout/stderr 第一行），失败为 None
    pub version: Option<String>,
    /// 可执行文件绝对路径，失败为 None
    pub path: Option<String>,
}

/// 在 PATH 中查找可执行文件。Windows 用 `where`，其他平台用 `which`。
/// 返回第一行的绝对路径；找不到返回 None。
async fn find_executable_in_path(executable: &str) -> Option<String> {
    #[cfg(windows)]
    let finder = "where";
    #[cfg(not(windows))]
    let finder = "which";

    let mut cmd = tokio::process::Command::new(finder);
    cmd.arg(executable);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = cmd.output().await.ok()?;

    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|s| s.trim().to_string())
}

/// 检测命令行工具是否已安装。
///
/// 设计：通用命令，不耦合 software.json。前端按条目 detect 字段传入 executable+args。
/// 1. PATH 中找不到 executable → installed=false, path=None
/// 2. 找到 executable 但执行失败/非零退出 → installed=false, path=Some
/// 3. 执行成功 → installed=true, version=stdout/stderr 第一行, path=Some
///
/// 注意：部分工具（如 `java -version`）将版本号输出到 stderr，
/// 这里同时检查 stdout 与 stderr 取第一行作为版本号。
#[tauri::command]
pub async fn check_software_installed(
    executable: String,
    args: Vec<String>,
) -> Result<InstalledStatus, String> {
    if executable.trim().is_empty() {
        return Err(AppError::Validation("可执行文件名不能为空".to_string()).to_string());
    }

    let path = find_executable_in_path(&executable).await;

    if path.is_none() {
        return Ok(InstalledStatus {
            installed: false,
            version: None,
            path: None,
        });
    }

    // 执行命令获取版本号；超时 5 秒避免长时间挂起
    let mut version_cmd = tokio::process::Command::new(&executable);
    version_cmd.args(&args);
    #[cfg(target_os = "windows")]
    version_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let run_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        version_cmd.output(),
    )
    .await;

    let version = match run_result {
        Ok(Ok(out)) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            stdout
                .lines()
                .next()
                .or_else(|| stderr.lines().next())
                .map(|s| s.trim().to_string())
        }
        _ => None,
    };

    Ok(InstalledStatus {
        // 找到可执行文件即视为"已安装"——
        // 即使 --version 命令失败，至少二进制文件存在于 PATH 中
        installed: version.is_some(),
        version,
        path,
    })
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

    /// 真实 PATH 检测（需本机存在对应可执行文件，默认跳过）：
    /// 验证 check_software_installed 命令的端到端行为。
    /// 运行：cargo test software -- --ignored
    #[tokio::test]
    #[ignore]
    async fn check_existing_command_version() {
        // cmd 内置 echo 在 Windows 上能稳定返回 0
        let status = super::check_software_installed("cmd".to_string(), vec!["/c".to_string(), "echo hello".to_string()])
            .await
            .expect("命令不应返回 Err");
        assert!(status.installed);
        assert_eq!(status.version.as_deref(), Some("hello"));
        assert!(status.path.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn check_nonexistent_command_returns_not_installed() {
        let status = super::check_software_installed(
            "definitely-not-existing-xyz-12345".to_string(),
            vec!["--version".to_string()],
        )
        .await
        .expect("命令不应返回 Err");
        assert!(!status.installed);
        assert!(status.path.is_none());
    }

    #[tokio::test]
    async fn check_empty_executable_returns_validation_error() {
        let result = super::check_software_installed("  ".to_string(), vec![]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("可执行文件名不能为空"));
    }
}
