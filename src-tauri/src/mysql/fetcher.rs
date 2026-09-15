use super::super::{download_control, logger, types::{MySQLVersionInfo, MySQLPackageOption}};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;

/// MySQL Installer 归档下载基址（历史版本通道）
const MYSQL_INSTALLER_BASE: &str = "https://cdn.mysql.com/archives/mysql-installer/";
/// 当前 GA 下载库基址（最新版本通道）
const MYSQL_INSTALLER_DOWNLOADS_BASE: &str = "https://cdn.mysql.com/Downloads/MySQLInstaller/";

/// 共享 HTTP 客户端：仅设连接超时，流式下载不受总超时限制
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

/// 内置 MySQL 版本号列表（编译期嵌入 JSON，零网络依赖）
static MYSQL_VERSIONS_JSON: &str = include_str!("../../mysql_versions.json");

/// 包声明（对象条目中的 packages 元素）
#[derive(serde::Deserialize)]
struct PackageSpec {
    /// 包类型：offline / online
    #[serde(rename = "type")]
    package_type: String,
    /// 安装器构建号（文件名末段 .x），默认 0
    #[serde(default = "default_build")]
    build: String,
}

fn default_build() -> String {
    "0".to_string()
}

/// 版本条目：字符串 = 默认包结构（离线 .0 + 在线 .0，archives 通道）；对象 = 可显式声明下载通道与包列表
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum VersionEntry {
    Simple(String),
    Detailed {
        version: String,
        /// 下载通道：archives（默认，历史归档）/ downloads（当前 GA 下载库）
        #[serde(default)]
        channel: String,
        #[serde(default)]
        packages: Vec<PackageSpec>,
    },
}

/// 内置版本清单：versions 为全部可选版本；offlineOnly 中的条目仅提供离线版安装器
#[derive(serde::Deserialize)]
struct MysqlVersionCatalog {
    #[serde(default)]
    versions: Vec<VersionEntry>,
    #[serde(rename = "offlineOnly", default)]
    offline_only: Vec<String>,
}

/// 根据版本号、包类型、安装器构建号与下载通道拼接链接
/// 离线版：mysql-installer-community-{version}.{build}.msi
/// 在线版：mysql-installer-web-community-{version}.{build}.msi
/// 末段 .x 为安装器构建号，绝大多数版本固定 0；5.5.33 存在 0/2 两个构建，已在 JSON 显式声明
fn build_download_link(version: &str, package_type: &str, build: &str, channel: &str) -> String {
    let base = match channel {
        "downloads" => MYSQL_INSTALLER_DOWNLOADS_BASE,
        _ => MYSQL_INSTALLER_BASE,
    };
    let filename = match package_type {
        "online" => format!("mysql-installer-web-community-{}.{}.msi", version, build),
        _ => format!("mysql-installer-community-{}.{}.msi", version, build),
    };
    format!("{}{}", base, filename)
}

/// 包类型标识解析："offline"/"online" 或带构建号后缀 "offline-0"/"offline-2"
fn parse_package_spec(package_type: &str) -> (String, String) {
    match package_type.split_once('-') {
        Some((t, b)) => (t.to_string(), b.to_string()),
        None => (package_type.to_string(), "0".to_string()),
    }
}

/// 构造包选项；multi_build 时显示名附加构建号、包类型标识带构建号后缀保证唯一
fn build_package(version: &str, package_type: &str, build: &str, multi_build: bool, channel: &str) -> MySQLPackageOption {
    let display = if package_type == "online" { "在线版" } else { "离线版" };
    MySQLPackageOption {
        package_type: if multi_build {
            format!("{}-{}", package_type, build)
        } else {
            package_type.to_string()
        },
        display_name: if multi_build {
            format!("{}（构建 .{}）", display, build)
        } else {
            display.to_string()
        },
        download_link: build_download_link(version, package_type, build, channel),
        size: 0,
    }
}

/// 解析内置版本清单，为每个版本构造包选项列表
pub fn get_available_mysql_versions() -> Result<Vec<MySQLVersionInfo>, String> {
    let catalog: MysqlVersionCatalog = serde_json::from_str(MYSQL_VERSIONS_JSON)
        .map_err(|e| format!("解析 MySQL 版本列表失败: {}", e))?;
    let offline_only: HashSet<&str> = catalog.offline_only.iter().map(|s| s.as_str()).collect();

    // 默认包结构：离线 .0，未标记 offlineOnly 再加在线 .0
    let default_packages = |version: &str, channel: &str| -> Vec<MySQLPackageOption> {
        let mut p = vec![build_package(version, "offline", "0", false, channel)];
        if !offline_only.contains(version) {
            p.push(build_package(version, "online", "0", false, channel));
        }
        p
    };

    let mut result = Vec::new();
    for entry in catalog.versions {
        match entry {
            VersionEntry::Simple(version) => {
                let packages = default_packages(&version, "archives");
                result.push(MySQLVersionInfo { version, packages });
            }
            VersionEntry::Detailed { version, channel, packages: specs } => {
                let packages = if specs.is_empty() {
                    // 未显式声明 packages 时按默认结构生成
                    default_packages(&version, &channel)
                } else {
                    // 同一类型出现多个构建号时，显示名附加构建号以便区分
                    let mut counts: HashMap<&str, usize> = HashMap::new();
                    for s in &specs {
                        *counts.entry(s.package_type.as_str()).or_default() += 1;
                    }
                    specs
                        .iter()
                        .map(|s| {
                            build_package(
                                &version,
                                &s.package_type,
                                &s.build,
                                counts[s.package_type.as_str()] > 1,
                                &channel,
                            )
                        })
                        .collect()
                };
                result.push(MySQLVersionInfo { version, packages });
            }
        }
    }

    Ok(result)
}

/// 查询版本所属下载通道："archives"（默认）/ "downloads"
fn version_channel(version: &str) -> String {
    if let Ok(catalog) = serde_json::from_str::<MysqlVersionCatalog>(MYSQL_VERSIONS_JSON) {
        for entry in catalog.versions {
            if let VersionEntry::Detailed { version: v, channel, .. } = entry {
                if v == version && !channel.is_empty() {
                    return channel;
                }
            }
        }
    }
    "archives".to_string()
}

/// 从下载链接中提取文件名
fn extract_filename(link: &str) -> String {
    link.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "mysql-installer.msi".to_string())
}

async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

/// 下载 MySQL 安装包：根据版本号和包类型标识（offline/online，可带构建号后缀如 offline-2）拼接链接
pub async fn download_mysql(
    app_handle: AppHandle,
    version: String,
    package_type: String,
    window: tauri::Window,
) -> Result<String, String> {
    if version.is_empty() || package_type.is_empty() {
        return Err("版本号或包类型为空".into());
    }

    // 校验版本号是否在内置列表中
    let available = get_available_mysql_versions()?;
    if !available.iter().any(|v| v.version == version) {
        return Err(format!("未找到 MySQL {} 版本", version));
    }

    let task_id = format!("mysql:{}-{}", version, package_type);
    logger::info(
        &app_handle,
        &format!("开始下载 MySQL {} ({})", version, package_type),
    );

    let (pkg_kind, build) = parse_package_spec(&package_type);
    let channel = version_channel(&version);
    let download_link = build_download_link(&version, &pkg_kind, &build, &channel);
    let filename = extract_filename(&download_link);
    let download_dir = get_download_dir().await;
    let file_path = download_dir.join(&filename);

    let (actual_size, declared_size) = download_control::download_file(
        &app_handle,
        &window,
        &task_id,
        &download_link,
        &file_path,
        &version,
        &HTTP_CLIENT,
        3,
    )
    .await?;

    // 大小校验
    if declared_size > 0 && actual_size != declared_size {
        let err = format!(
            "文件大小不完整（实际 {} / 预期 {} 字节），可能下载中断",
            actual_size, declared_size
        );
        logger::error(&app_handle, &err);
        let _ = tokio::fs::remove_file(&file_path).await;
        download_control::emit_failed(&window, &task_id, &version, actual_size, declared_size);
        return Err(err);
    }

    // MD5 自动校验：MySQL CDN 提供 .md5 侧车文件
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
    match HTTP_CLIENT.get(&md5_sidecar).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            // 格式："<md5>  <filename>"
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
    logger::info(
        &app_handle,
        &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size),
    );
    Ok(file_path.to_string_lossy().to_string())
}
