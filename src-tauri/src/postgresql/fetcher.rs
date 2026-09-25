use super::super::{download_control, http_client, logger, remote_version_cache, types::{PostgresqlVersionInfo}};
use std::sync::LazyLock;
use std::path::PathBuf;
use tauri::AppHandle;

/// EnterpriseDB 安装器下载基址
const POSTGRESQL_INSTALLER_BASE: &str = "https://get.enterprisedb.com/postgresql/";

/// 远程版本数据 URL
const REMOTE_VERSIONS_URL: &str = "https://raw.githubusercontent.com/nicepkg/dev-tools/main/src/data/postgresql_versions.json";

/// 内置 PostgreSQL 版本号列表（编译期嵌入 JSON，作为远程拉取失败时的兜底）
static POSTGRESQL_VERSIONS_JSON: &str = include_str!("../../../src/data/postgresql_versions.json");

/// 内置版本清单：versions 为字符串数组，每项格式 "version-build"（如 "16.15-3"）
#[derive(serde::Deserialize)]
struct PostgresVersionCatalog {
    versions: Vec<String>,
}

/// 解析后的版本目录缓存：JSON 只解析一次
static CATALOG: LazyLock<PostgresVersionCatalog> = LazyLock::new(|| {
    serde_json::from_str(POSTGRESQL_VERSIONS_JSON)
        .expect("内置 postgresql_versions.json 解析失败（编译期嵌入，不应出错）")
});

/// 拼接下载链接：postgresql-{version-build}-windows-x64.exe
fn build_download_link(version_build: &str) -> String {
    format!("{}postgresql-{}-windows-x64.exe", POSTGRESQL_INSTALLER_BASE, version_build)
}

/// 从缓存目录生成版本信息列表
pub fn get_available_postgresql_versions() -> Result<Vec<PostgresqlVersionInfo>, String> {
    let catalog = &*CATALOG;
    build_version_list(catalog)
}

/// 异步版本：优先从远程拉取最新数据，带回缓存和嵌入数据回退
pub async fn get_available_postgresql_versions_async(
    app_handle: AppHandle,
) -> Result<Vec<PostgresqlVersionInfo>, String> {
    let json_str = remote_version_cache::fetch_version_data(
        "postgresql_versions",
        REMOTE_VERSIONS_URL,
        POSTGRESQL_VERSIONS_JSON,
        &app_handle,
    )
    .await;

    let catalog: PostgresVersionCatalog = serde_json::from_str(&json_str)
        .map_err(|e| format!("PostgreSQL 版本数据解析失败: {}", e))?;
    build_version_list(&catalog)
}

fn build_version_list(catalog: &PostgresVersionCatalog) -> Result<Vec<PostgresqlVersionInfo>, String> {
    let result = catalog
        .versions
        .iter()
        .map(|v| PostgresqlVersionInfo {
            download_link: build_download_link(v),
            version: v.clone(),
            size: 0,
        })
        .collect();

    Ok(result)
}

/// 从下载链接中提取文件名
fn extract_filename(link: &str) -> String {
    link.rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "postgresql-installer.exe".to_string())
}

async fn get_download_dir() -> PathBuf {
    let dir = download_control::devtools_download_dir();
    let _ = tokio::fs::create_dir_all(&dir).await;
    dir
}

/// 下载 PostgreSQL 安装包：根据版本号（含 build，如 "16.15-3"）拼接链接
pub async fn download_postgresql(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<String, String> {
    if version.is_empty() {
        return Err("版本号为空".into());
    }

    // 校验版本号是否在内置列表中
    let available = get_available_postgresql_versions_async(app_handle.clone()).await?;
    if !available.iter().any(|v| v.version == version) {
        return Err(format!("未找到 PostgreSQL {} 版本", version));
    }

    let task_id = format!("postgresql:{}", version);
    logger::info(
        &app_handle,
        &format!("开始下载 PostgreSQL {}", version),
    );

    let download_link = build_download_link(&version);
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
        http_client::default_client(),
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

    // EDB CDN 不提供 MD5/SHA 侧车文件，仅记录本地哈希到日志
    if let Ok(local_hash) = download_control::compute_file_hash(&file_path).await {
        logger::info(&app_handle, &format!("安装包 SHA-256: {}", local_hash));
    }

    download_control::emit_completed(&window, &task_id, &version, actual_size, declared_size);
    logger::info(
        &app_handle,
        &format!("下载完成: {}（{} 字节）", file_path.display(), actual_size),
    );
    Ok(file_path.to_string_lossy().to_string())
}
