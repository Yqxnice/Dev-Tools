use super::super::{
    download_control,
    http_client,
    logger,
    types::{JetBrainsVersionDetail, JetBrainsVersionInfo, JetBrainsPackageOption},
};
use std::path::PathBuf;
use tauri::AppHandle;

/// JetBrains 全家桶产品代号（API code）与展示名映射
pub static JETBRAINS_PRODUCTS: &[(&str, &str)] = &[
    ("IIU", "IntelliJ IDEA Ultimate"),
    ("IIC", "IntelliJ IDEA Community"),
    ("PCP", "PyCharm Professional"),
    ("PCC", "PyCharm Community"),
    ("WS", "WebStorm"),
    ("GO", "GoLand"),
    ("CL", "CLion"),
    ("DG", "DataGrip"),
    ("PS", "PhpStorm"),
    ("RM", "RubyMine"),
    ("RR", "RustRover"),
    ("RD", "Rider"),
];

/// JetBrains Data Services API URL
/// `latest_only = true` 返回最新稳定版；`false` 返回全部发行版
fn releases_api_url(code: &str, latest_only: bool) -> String {
    let latest = if latest_only { "&latest=true" } else { "" };
    format!(
        "https://data.services.jetbrains.com/products/releases?code={}&type=release{}",
        code, latest
    )
}

/// 反序列化 API 响应结构：{ "<CODE>": [{ version, date, downloads: {...} }] }
#[derive(serde::Deserialize)]
struct ApiReleases {
    #[serde(flatten)]
    map: std::collections::HashMap<String, Vec<ApiRelease>>,
}

#[derive(serde::Deserialize, Clone)]
struct ApiRelease {
    version: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    downloads: ApiDownloads,
}

/// downloads 字段：windows(x64 exe) / windowsZip(便携 zip) / windowsARM64(ARM64 exe)
#[derive(serde::Deserialize, Clone, Default)]
struct ApiDownloads {
    #[serde(default)]
    windows: Option<ApiDownload>,
    #[serde(default, rename = "windowsZip")]
    windows_zip: Option<ApiDownload>,
    #[serde(default, rename = "windowsARM64")]
    windows_arm64: Option<ApiDownload>,
}

#[derive(serde::Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct ApiDownload {
    #[serde(default)]
    link: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    checksum_link: String,
}

/// 将 ApiDownloads 转换为前端可选的包类型列表
fn collect_packages(downloads: &ApiDownloads) -> Vec<JetBrainsPackageOption> {
    let mut packages = Vec::new();
    if let Some(dl) = &downloads.windows {
        if !dl.link.is_empty() {
            packages.push(JetBrainsPackageOption {
                package_type: "windows".to_string(),
                display_name: "Windows x64 (安装程序)".to_string(),
                download_link: dl.link.clone(),
                size: dl.size,
                checksum_link: dl.checksum_link.clone(),
            });
        }
    }
    if let Some(dl) = &downloads.windows_zip {
        if !dl.link.is_empty() {
            packages.push(JetBrainsPackageOption {
                package_type: "windowsZip".to_string(),
                display_name: "Windows x64 (便携 zip)".to_string(),
                download_link: dl.link.clone(),
                size: dl.size,
                checksum_link: dl.checksum_link.clone(),
            });
        }
    }
    if let Some(dl) = &downloads.windows_arm64 {
        if !dl.link.is_empty() {
            packages.push(JetBrainsPackageOption {
                package_type: "windowsARM64".to_string(),
                display_name: "Windows ARM64 (安装程序)".to_string(),
                download_link: dl.link.clone(),
                size: dl.size,
                checksum_link: dl.checksum_link.clone(),
            });
        }
    }
    packages
}

/// 拉取所有产品的最新发行版列表（概览用，每产品仅一条最新版）。
/// 单产品失败不中断整体查询（与检测命令一致的"尽力而为"语义）。
pub async fn get_available_jetbrains_versions(
    app_handle: AppHandle,
) -> Result<Vec<JetBrainsVersionInfo>, String> {
    logger::info(&app_handle, "正在获取 JetBrains 可用版本列表...");

    let mut all: Vec<JetBrainsVersionInfo> = Vec::new();
    let mut failures = 0u32;

    for (code, name) in JETBRAINS_PRODUCTS {
        match fetch_product_releases(code, true).await {
            Ok(list) => {
                if let Some(rel) = list.into_iter().next() {
                    // 概览默认取 windows 包
                    let dl = rel.downloads.windows.clone().unwrap_or_default();
                    if !dl.link.is_empty() {
                        all.push(JetBrainsVersionInfo {
                            product_code: code.to_string(),
                            product_name: name.to_string(),
                            version: rel.version,
                            date: rel.date,
                            download_link: dl.link,
                            size: dl.size,
                            checksum_link: dl.checksum_link,
                        });
                    }
                }
            }
            Err(e) => {
                failures += 1;
                logger::warn(
                    &app_handle,
                    &format!("获取 {} 版本失败: {}", name, e),
                );
            }
        }
    }

    if all.is_empty() {
        return Err(format!(
            "所有 JetBrains 产品版本查询均失败（共 {} 个产品）",
            JETBRAINS_PRODUCTS.len()
        ));
    }

    all.sort_by(|a, b| a.product_name.cmp(&b.product_name));
    logger::info(
        &app_handle,
        &format!("获取完成，共 {} 个产品最新版（{} 个失败）", all.len(), failures),
    );
    Ok(all)
}

/// 拉取指定产品的全部发行版列表（含各版本可用的包类型）
pub async fn get_jetbrains_product_versions(
    app_handle: AppHandle,
    product_code: String,
) -> Result<Vec<JetBrainsVersionDetail>, String> {
    let product_name = JETBRAINS_PRODUCTS
        .iter()
        .find(|(code, _)| *code == product_code)
        .map(|(_, name)| name.to_string())
        .ok_or_else(|| format!("未知产品代号: {}", product_code))?;

    logger::info(&app_handle, &format!("正在获取 {} 全部版本...", product_name));

    let releases = fetch_product_releases(&product_code, false).await?;

    let versions: Vec<JetBrainsVersionDetail> = releases
        .into_iter()
        .map(|rel| {
            let packages = collect_packages(&rel.downloads);
            JetBrainsVersionDetail {
                version: rel.version,
                date: rel.date,
                packages,
            }
        })
        .filter(|v| !v.packages.is_empty())
        .collect();

    if versions.is_empty() {
        return Err(format!("{} 未找到任何可用版本", product_name));
    }

    logger::info(
        &app_handle,
        &format!("获取完成，{} 共 {} 个版本", product_name, versions.len()),
    );
    Ok(versions)
}

/// 拉取单个产品的发行版列表
async fn fetch_product_releases(code: &str, latest_only: bool) -> Result<Vec<ApiRelease>, String> {
    let url = releases_api_url(code, latest_only);
    let resp = http_client::default_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let mut body: ApiReleases = resp
        .json()
        .await
        .map_err(|e| format!("解析 JSON 失败: {}", e))?;

    Ok(body.map.remove(code).unwrap_or_default())
}

/// 从下载链接中提取文件名（末段）
fn extract_filename(link: &str) -> String {
    link.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "jetbrains-installer.exe".to_string())
}

/// 下载目录：复用公共路径函数，确保与 Python fetcher / lib.rs 命令一致
async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

/// 校验安装包：优先对照 JetBrains 提供的 checksumLink 侧车文件
async fn verify_installer(
    app_handle: &AppHandle,
    checksum_link: &str,
    actual_size: u64,
    declared_size: u64,
    actual_hash: &str,
) -> Result<(), String> {
    if declared_size > 0 && actual_size != declared_size {
        return Err(format!(
            "文件大小不完整（实际 {} 字节 / 预期 {} 字节），可能下载中断",
            actual_size, declared_size
        ));
    }

    if checksum_link.is_empty() {
        logger::info(
            app_handle,
            &format!("未提供 checksum，安装包 SHA256 仅供核对: {}", actual_hash),
        );
        return Ok(());
    }

    match http_client::default_client().get(checksum_link).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            // JetBrains checksum 文件格式通常为 "<hash>  <filename>" 或纯 hash
            let expected = body
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .to_lowercase();
            if expected.len() == 64 && expected == actual_hash {
                logger::info(app_handle, "安装包 SHA256 校验通过（与官方 checksum 一致）");
            } else if expected.len() == 64 {
                return Err(
                    "SHA256 校验失败：文件哈希与官方 checksum 不一致（文件已删除，请重试）".to_string(),
                );
            } else {
                logger::info(
                    app_handle,
                    &format!("checksum 文件无法解析，安装包 SHA256 为: {}", actual_hash),
                );
            }
        }
        _ => {
            logger::info(
                app_handle,
                &format!("checksum 链接不可用，安装包 SHA256 为: {}", actual_hash),
            );
        }
    }
    Ok(())
}

/// 根据包类型从 release 中提取对应的下载信息
fn get_package_download<'a>(
    release: &'a ApiRelease,
    package_type: &str,
) -> Result<&'a ApiDownload, String> {
    let dl = match package_type {
        "windows" => release.downloads.windows.as_ref(),
        "windowsZip" => release.downloads.windows_zip.as_ref(),
        "windowsARM64" => release.downloads.windows_arm64.as_ref(),
        _ => None,
    };
    dl.ok_or_else(|| format!("未提供 {} 包类型", package_type))
}

/// 下载主流程：支持暂停/继续/取消，校验 SHA256
pub async fn download_jetbrains(
    app_handle: AppHandle,
    product_code: String,
    version: String,
    package_type: String,
    window: tauri::Window,
) -> Result<String, String> {
    if product_code.is_empty() || version.is_empty() || package_type.is_empty() {
        return Err("产品代号、版本号或包类型为空".into());
    }

    let task_id = format!("jetbrains:{}-{}-{}", product_code, version, package_type);

    logger::info(
        &app_handle,
        &format!("开始下载 JetBrains {} {} ({})", product_code, version, package_type),
    );

    // 重新查询 API 获取该版本的下载链接/大小/checksum（避免前端伪造 URL）
    let releases = fetch_product_releases(&product_code, false)
        .await
        .map_err(|e| format!("查询 {} 版本失败: {}", product_code, e))?;
    let release = releases
        .iter()
        .find(|r| r.version == version)
        .ok_or_else(|| format!("未在 API 返回中找到 {} 的 {} 版本", product_code, version))?;
    let download = get_package_download(release, &package_type)?;
    if download.link.is_empty() {
        return Err(format!("{} 的 {} 版本下载链接为空", product_code, version));
    }

    let filename = extract_filename(&download.link);
    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(&filename);

    // 下载主流程：暂停/继续/取消/重试由 download_control 统一处理
    let (actual_size, _) = download_control::download_file(
        &app_handle,
        &window,
        &task_id,
        &download.link,
        &file_path,
        &version,
        http_client::default_client(),
        3,
    )
    .await?;

    // 计算 SHA256（非增量式，支持暂停/继续后的完整校验）
    let hash = match download_control::compute_file_hash(&file_path).await {
        Ok(h) => h,
        Err(e) => {
            logger::error(&app_handle, &format!("计算文件哈希失败: {}", e));
            let _ = tokio::fs::remove_file(&file_path).await;
            download_control::emit_failed(&window, &task_id, &version, actual_size, download.size);
            return Err(e);
        }
    };

    // 校验
    if let Err(verify_err) = verify_installer(
        &app_handle,
        &download.checksum_link,
        actual_size,
        download.size,
        &hash,
    )
    .await
    {
        logger::error(&app_handle, &format!("安装包校验失败: {}", verify_err));
        let _ = tokio::fs::remove_file(&file_path).await;
        download_control::emit_failed(&window, &task_id, &version, actual_size, download.size);
        return Err(verify_err);
    }

    // 发送完成终态事件
    download_control::emit_completed(&window, &task_id, &version, actual_size, download.size);
    logger::info(
        &app_handle,
        &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size),
    );
    Ok(file_path.to_string_lossy().to_string())
}
