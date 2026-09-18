use super::super::{download_control, http_client, logger, types::{MySQLVersionInfo, MySQLPackageOption}};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::AppHandle;

// ── 下载基址 ──
/// 独立服务器归档：archives/mysql-{major}.{minor}/mysql-{version}-winx64.{msi|zip}
const SERVER_ARCHIVES_BASE: &str = "https://cdn.mysql.com/archives/";
/// Installer 归档：archives/mysql-installer/mysql-installer-{community|web-community}-{version}.0.msi
const INSTALLER_ARCHIVES_BASE: &str = "https://cdn.mysql.com/archives/mysql-installer/";
/// Installer 当前 GA：Downloads/MySQLInstaller/（8.0.46、5.7.44 等仍在 GA 通道）
const INSTALLER_DOWNLOADS_BASE: &str = "https://cdn.mysql.com/Downloads/MySQLInstaller/";

/// 内置 MySQL 版本清单（编译期嵌入）
static MYSQL_VERSIONS_JSON: &str = include_str!("../../../src/data/mysql_versions.json");

// ── JSON 数据结构 ──
#[derive(serde::Deserialize)]
struct CategoryEntry {
    name: String,
    /// "server"（独立安装包）或 "installer"（一体化管理器）
    #[serde(default)]
    mode: String,
    versions: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MysqlVersionCatalog {
    categories: Vec<CategoryEntry>,
    #[serde(default)]
    downloads_channel: Vec<String>,
    #[serde(default)]
    offline_only: Vec<String>,
}

/// 解析后的版本目录缓存：JSON 只解析一次，所有查询共享同一份数据。
/// 避免每次调用 get_available_mysql_versions 时重复解析。
static CATALOG: Lazy<MysqlVersionCatalog> = Lazy::new(|| {
    serde_json::from_str(MYSQL_VERSIONS_JSON)
        .expect("内置 mysql_versions.json 解析失败（编译期嵌入，不应出错）")
});

// ── URL 生成 ──
/// 取版本号前两段作为目录段：9.4.0 → 9.4
fn major_minor(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    match (parts.first(), parts.get(1)) {
        (Some(maj), Some(min)) => format!("{}.{}", maj, min),
        _ => version.to_string(),
    }
}

/// 独立服务器链接：archives/mysql-{major}.{minor}/mysql-{version}-winx64.{ext}
fn server_link(version: &str, ext: &str) -> String {
    format!(
        "{}mysql-{}/mysql-{}-winx64.{}",
        SERVER_ARCHIVES_BASE,
        major_minor(version),
        version,
        ext
    )
}

/// Installer 链接
/// archives: archives/mysql-installer/mysql-installer-{kind}-{version}.0.msi
/// downloads: Downloads/MySQLInstaller/mysql-installer-{kind}-{version}.0.msi
fn installer_link(version: &str, kind: &str, use_downloads: bool) -> String {
    let base = if use_downloads {
        INSTALLER_DOWNLOADS_BASE
    } else {
        INSTALLER_ARCHIVES_BASE
    };
    let prefix = match kind {
        "online" => "mysql-installer-web-community",
        _ => "mysql-installer-community",
    };
    format!("{}{}-{}.0.msi", base, prefix, version)
}

// ── 包选项生成 ──
/// 独立服务器：MSI 安装包 + ZIP 压缩包
fn server_packages(version: &str) -> Vec<MySQLPackageOption> {
    vec![
        MySQLPackageOption {
            package_type: "msi".to_string(),
            display_name: "安装包 (MSI)".to_string(),
            download_link: server_link(version, "msi"),
            size: 0,
        },
        MySQLPackageOption {
            package_type: "zip".to_string(),
            display_name: "压缩包 (ZIP)".to_string(),
            download_link: server_link(version, "zip"),
            size: 0,
        },
    ]
}

/// Installer 管理器：离线版 + 在线版（offlineOnly 中的版本仅离线）
fn installer_packages(version: &str, use_downloads: bool, offline_only: bool) -> Vec<MySQLPackageOption> {
    let mut pkgs = vec![MySQLPackageOption {
        package_type: "offline".to_string(),
        display_name: "离线版".to_string(),
        download_link: installer_link(version, "offline", use_downloads),
        size: 0,
    }];
    if !offline_only {
        pkgs.push(MySQLPackageOption {
            package_type: "online".to_string(),
            display_name: "在线版".to_string(),
            download_link: installer_link(version, "online", use_downloads),
            size: 0,
        });
    }
    pkgs
}

// ── 核心：从缓存目录生成 Vec<MySQLVersionInfo> ──
pub fn get_available_mysql_versions() -> Result<Vec<MySQLVersionInfo>, String> {
    let catalog = &*CATALOG;
    let dl_set: HashSet<&str> = catalog.downloads_channel.iter().map(|s| s.as_str()).collect();
    let offline_set: HashSet<&str> = catalog.offline_only.iter().map(|s| s.as_str()).collect();

    let mut result = Vec::new();
    for cat in &catalog.categories {
        let mode = if cat.mode == "server" { "server" } else { "installer" };
        for version in &cat.versions {
            let use_dl = dl_set.contains(version.as_str());
            let offline_only = offline_set.contains(version.as_str());
            let packages = if mode == "server" {
                server_packages(version)
            } else {
                installer_packages(version, use_dl, offline_only)
            };
            result.push(MySQLVersionInfo {
                mode: mode.to_string(),
                category: cat.name.clone(),
                version: version.clone(),
                packages,
            });
        }
    }
    Ok(result)
}

fn extract_filename(link: &str) -> String {
    link.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "mysql.msi".to_string())
}

async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

// ── 下载 ──
pub async fn download_mysql(
    app_handle: AppHandle,
    version: String,
    package_type: String,
    window: tauri::Window,
) -> Result<String, String> {
    if version.is_empty() || package_type.is_empty() {
        return Err("版本号或包类型为空".into());
    }

    let available = get_available_mysql_versions()?;
    let pkg = available
        .iter()
        .find(|v| v.version == version)
        .and_then(|v| v.packages.iter().find(|p| p.package_type == package_type))
        .ok_or_else(|| format!("未找到 MySQL {} ({})", version, package_type))?;

    let task_id = format!("mysql:{}-{}", version, package_type);
    logger::info(&app_handle, &format!("开始下载 MySQL {} ({})", version, package_type));

    let download_link = pkg.download_link.clone();
    let filename = extract_filename(&download_link);
    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(&filename);

    let (actual_size, declared_size) = download_control::download_file(
        &app_handle, &window, &task_id, &download_link, &file_path, &version, http_client::default_client(), 3,
    ).await?;

    if declared_size > 0 && actual_size != declared_size {
        let err = format!("文件大小不完整（实际 {} / 预期 {} 字节），可能下载中断", actual_size, declared_size);
        logger::error(&app_handle, &err);
        let _ = tokio::fs::remove_file(&file_path).await;
        download_control::emit_failed(&window, &task_id, &version, actual_size, declared_size);
        return Err(err);
    }

    // MD5 侧车校验
    let actual_md5 = match download_control::compute_file_md5(&file_path).await {
        Ok(h) => h,
        Err(e) => {
            logger::error(&app_handle, &format!("计算文件 MD5 失败: {}", e));
            let _ = tokio::fs::remove_file(&file_path).await;
            download_control::emit_failed(&window, &task_id, &version, actual_size, declared_size);
            return Err(e);
        }
    };
    let md5_sidecar = format!("{}.md5", download_link);
    match http_client::default_client().get(&md5_sidecar).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            let expected = body.split_whitespace().next().unwrap_or("").trim().to_lowercase();
            if expected.len() == 32 && expected == actual_md5 {
                logger::info(&app_handle, "安装包 MD5 校验通过（与官方一致）");
            } else if expected.len() == 32 {
                let err = "MD5 校验失败：文件哈希与官方不一致（文件已删除，请重试）".to_string();
                logger::error(&app_handle, &err);
                let _ = tokio::fs::remove_file(&file_path).await;
                download_control::emit_failed(&window, &task_id, &version, actual_size, declared_size);
                return Err(err);
            } else {
                logger::info(&app_handle, &format!("MD5 侧车文件无法解析，安装包 MD5: {}", actual_md5));
            }
        }
        _ => {
            logger::info(&app_handle, &format!("MD5 侧车文件不可用，安装包 MD5: {}（可到 dev.mysql.com 人工核对）", actual_md5));
        }
    }

    download_control::emit_completed(&window, &task_id, &version, actual_size, declared_size);
    logger::info(&app_handle, &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size));
    Ok(file_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_catalog() {
        let catalog: MysqlVersionCatalog = serde_json::from_str(MYSQL_VERSIONS_JSON).unwrap();
        assert!(catalog.categories.len() >= 7);
        assert!(catalog.downloads_channel.contains(&"8.0.46".to_string()));
        assert!(catalog.downloads_channel.contains(&"5.7.44".to_string()));
        assert!(catalog.offline_only.contains(&"5.5.27".to_string()));
        assert!(catalog.offline_only.contains(&"5.6.10".to_string()));
    }

    #[test]
    fn server_link_pattern() {
        assert_eq!(
            server_link("9.4.0", "msi"),
            "https://cdn.mysql.com/archives/mysql-9.4/mysql-9.4.0-winx64.msi"
        );
        assert_eq!(
            server_link("8.4.3", "zip"),
            "https://cdn.mysql.com/archives/mysql-8.4/mysql-8.4.3-winx64.zip"
        );
    }

    #[test]
    fn installer_archives_link() {
        assert_eq!(
            installer_link("8.0.45", "offline", false),
            "https://cdn.mysql.com/archives/mysql-installer/mysql-installer-community-8.0.45.0.msi"
        );
        assert_eq!(
            installer_link("8.0.45", "online", false),
            "https://cdn.mysql.com/archives/mysql-installer/mysql-installer-web-community-8.0.45.0.msi"
        );
    }

    #[test]
    fn installer_downloads_link() {
        assert_eq!(
            installer_link("8.0.46", "offline", true),
            "https://cdn.mysql.com/Downloads/MySQLInstaller/mysql-installer-community-8.0.46.0.msi"
        );
    }

    #[test]
    fn versions_have_correct_mode_and_packages() {
        let versions = get_available_mysql_versions().unwrap();
        // Server: 9.4.0 有 MSI+ZIP
        let v = versions.iter().find(|v| v.version == "9.4.0").unwrap();
        assert_eq!(v.mode, "server");
        assert_eq!(v.packages.len(), 2);
        assert_eq!(v.packages[0].package_type, "msi");
        assert_eq!(v.packages[1].package_type, "zip");

        // Installer normal: 8.0.45 有 offline+online
        let v = versions.iter().find(|v| v.version == "8.0.45").unwrap();
        assert_eq!(v.mode, "installer");
        assert_eq!(v.packages.len(), 2);
        assert_eq!(v.packages[0].package_type, "offline");
        assert_eq!(v.packages[1].package_type, "online");

        // Installer offlineOnly: 5.5.60 只有 offline
        let v = versions.iter().find(|v| v.version == "5.5.60").unwrap();
        assert_eq!(v.mode, "installer");
        assert_eq!(v.packages.len(), 1);
        assert_eq!(v.packages[0].package_type, "offline");

        // Installer downloads channel: 8.0.46 用 Downloads 基址
        let v = versions.iter().find(|v| v.version == "8.0.46").unwrap();
        assert!(v.packages[0].download_link.contains("/Downloads/MySQLInstaller/"));
    }
}
