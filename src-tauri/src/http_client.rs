//! 共享 HTTP 客户端：避免各 fetcher 各自创建 reqwest::Client 导致连接池无法复用。
//!
//! - `default_client()`：用于版本列表查询、下载等常规请求（仅 connect_timeout，不限总超时）
//! - `browser_client()`：用于软件官网图标抓取（模拟浏览器 UA + 总超时 + 重定向）

use once_cell::sync::Lazy;
use std::time::Duration;

/// 浏览器 UA：部分官网（如 mysql.com 走 Akamai WAF）会校验 UA，必须伪装成 Chrome
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// 通用客户端：仅设置连接超时，下载场景不能有总超时（大文件慢网会被截断）
static DEFAULT_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

/// 浏览器客户端：用于抓取官网 HTML/图标，需要 UA 伪装 + 总超时 + 重定向跟随
static BROWSER_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(BROWSER_UA)
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

pub fn default_client() -> &'static reqwest::Client {
    &DEFAULT_CLIENT
}

pub fn browser_client() -> &'static reqwest::Client {
    &BROWSER_CLIENT
}
