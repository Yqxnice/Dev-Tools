use super::super::{logger, types::{AvailablePythonVersion, DownloadProgress, InstallProgress}};
use regex::Regex;
use reqwest;
use tauri::{AppHandle, Emitter};
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use futures_util::StreamExt;

static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"href="(\d+\.\d+\.\d+)/""#).unwrap());
static STABLE_VERSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+\.\d+\.\d+$").unwrap());

const MIRRORS: &[(&str, &str)] = &[
    ("华为云", "https://mirrors.huaweicloud.com/python/"),
    ("淘宝", "https://registry.npmmirror.com/binary.html?path=python/"),
    ("腾讯云", "https://mirrors.cloud.tencent.com/python/"),
    ("官方", "https://www.python.org/ftp/python/"),
];

async fn fetch_versions_from_mirror(mirror_url: &str) -> Result<Vec<String>, String> {
    let response = reqwest::get(mirror_url)
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP 错误: {}", response.status()));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let mut versions: Vec<String> = VERSION_REGEX
        .captures_iter(&html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    versions.sort_by(|a, b| {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
        b_parts.cmp(&a_parts)
    });

    Ok(versions)
}

pub async fn get_available_python_versions(
    app_handle: AppHandle,
) -> Result<Vec<AvailablePythonVersion>, String> {
    logger::info(&app_handle, "正在获取可用的 Python 版本...");

    for (_mirror_name, mirror_url) in MIRRORS {
        match fetch_versions_from_mirror(mirror_url).await {
            Ok(versions) => {
                let available_versions: Vec<AvailablePythonVersion> = versions
                    .into_iter()
                    .take(50)
                    .map(|version| AvailablePythonVersion {
                        is_stable: STABLE_VERSION_REGEX.is_match(&version),
                        version,
                        release_date: None,
                        download_urls: vec![
                            format!("{}windows/", mirror_url),
                        ],
                    })
                    .collect();

                logger::info(
                    &app_handle,
                    &format!("获取成功，共 {} 个版本", available_versions.len()),
                );
                return Ok(available_versions);
            }
            Err(e) => {
                logger::warn(
                    &app_handle,
                    &format!("获取失败: {}", e),
                );
            }
        }
    }

    Err("所有镜像源都无法访问".to_string())
}

fn get_system_architecture() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "amd64"
    }
    #[cfg(target_arch = "x86")]
    {
        "win32"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "arm64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
    {
        "amd64" // 默认为 amd64
    }
}

pub fn get_download_url(version: &str) -> String {
    // 默认使用华为云镜像
    let arch = get_system_architecture();
    format!(
        "https://mirrors.huaweicloud.com/python/{}/python-{}-{}.exe",
        version, version, arch
    )
}

fn get_download_dir() -> PathBuf {
    if let Some(dir) = dirs::desktop_dir() {
        // 下载到桌面
        let _ = std::fs::create_dir_all(&dir);
        dir
    } else if let Some(mut dir) = dirs::download_dir() {
        dir.push("PythonInstallers");
        let _ = std::fs::create_dir_all(&dir);
        dir
    } else {
        PathBuf::from(".")
    }
}

pub async fn download_python(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<PathBuf, String> {
    let download_url = get_download_url(&version);
    logger::info(&app_handle, &format!("开始下载 Python {}", version));
    
    let arch = get_system_architecture();
    let download_dir = get_download_dir();
    let file_path = download_dir.join(format!("python-{}-{}.exe", version, arch));
    
    let response = reqwest::get(&download_url)
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status()));
    }
    
    let total_size = response.content_length().unwrap_or(0);
    let mut file = File::create(&file_path)
        .map_err(|e| format!("创建文件失败: {}", e))?;
    
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载数据失败: {}", e))?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).map_err(|e| format!("写入文件失败: {}", e))?;
        
        let percentage = if total_size > 0 {
            (downloaded as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };
        
        let progress = DownloadProgress {
            version: version.clone(),
            downloaded,
            total: total_size,
            percentage,
            status: "下载中".to_string(),
        };
        
        window.emit("download_progress", &progress)
            .map_err(|e| format!("发送进度事件失败: {}", e))?;
    }
    
    logger::info(&app_handle, "下载完成");
    Ok(file_path)
}

pub async fn install_python(
    app_handle: AppHandle,
    installer_path: String,
    version: String,
    install_path: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    logger::info(&app_handle, &format!("开始安装 Python {}", version));
    
    let emit_progress = |phase: &str, message: &str, percentage: u32| {
        let progress = InstallProgress {
            version: version.clone(),
            phase: phase.to_string(),
            message: message.to_string(),
            percentage,
            completed: false,
            success: false,
            error: None,
        };
        window.emit("install_progress", &progress).ok();
    };
    
    emit_progress("准备", "准备安装环境", 5);
    
    // 静默安装参数
    let mut install_args = vec![
        "/quiet".to_string(),
        "InstallAllUsers=0".to_string(),
        "PrependPath=1".to_string(),
        "Include_test=0".to_string(),
        "Include_launcher=1".to_string(),
        "AssociateFiles=1".to_string(),
    ];
    
    // 如果指定了自定义安装路径，添加安装路径参数
    if let Some(ref path) = install_path {
        install_args.push(format!("TargetDir={}", path));
    }
    
    emit_progress("安装", "正在运行安装程序", 20);
    
    // 直接使用 tokio::process 运行，不使用超时限制
    use tokio::process::Command;
    let result = Command::new(&installer_path)
        .args(&install_args)
        .output()
        .await;
    
    emit_progress("安装", "安装程序执行中", 60);
    
    match result {
        Ok(output) if output.status.success() || output.status.code() == Some(1641) || output.status.code() == Some(3010) => {
            // 0 = 成功, 1641 = 需要重启, 3010 = 已安装但需要重启
            logger::info(&app_handle, &format!("Python {} 安装成功", version));
            
            let final_progress = InstallProgress {
                version: version.clone(),
                phase: "完成".to_string(),
                message: "Python 安装成功!".to_string(),
                percentage: 100,
                completed: true,
                success: true,
                error: None,
            };
            window.emit("install_progress", &final_progress)
                .map_err(|e| format!("发送进度事件失败: {}", e))?;
            
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            let error_msg = format!("安装失败 (退出码: {}): {}", exit_code, stderr);
            logger::error(&app_handle, &error_msg);
            
            let final_progress = InstallProgress {
                version: version.clone(),
                phase: "失败".to_string(),
                message: error_msg.clone(),
                percentage: 100,
                completed: true,
                success: false,
                error: Some(error_msg.clone()),
            };
            window.emit("install_progress", &final_progress).ok();
            
            Err(error_msg)
        }
        Err(e) => {
            let error_msg = format!("执行安装程序失败: {}", e);
            logger::error(&app_handle, &error_msg);
            
            let final_progress = InstallProgress {
                version: version.clone(),
                phase: "失败".to_string(),
                message: error_msg.clone(),
                percentage: 100,
                completed: true,
                success: false,
                error: Some(error_msg.clone()),
            };
            window.emit("install_progress", &final_progress).ok();
            
            Err(error_msg)
        }
    }
}

// 只下载 Python 安装包
pub async fn download_python_only(
    app_handle: AppHandle,
    version: String,
    window: tauri::Window,
) -> Result<String, String> {
    let installer_path = download_python(app_handle.clone(), version.clone(), window.clone()).await?;
    
    // 发送完成进度事件
    let final_progress = InstallProgress {
        version: version.clone(),
        phase: "完成".to_string(),
        message: "下载完成!".to_string(),
        percentage: 100,
        completed: true,
        success: true,
        error: None,
    };
    window.emit("install_progress", &final_progress)
        .map_err(|e| format!("发送进度事件失败: {}", e))?;
    
    Ok(installer_path.to_str().unwrap().to_string())
}

pub async fn download_and_install_python(
    app_handle: AppHandle,
    version: String,
    install_path: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    // 1. 下载
    let installer_path = download_python(app_handle.clone(), version.clone(), window.clone()).await?;
    
    // 2. 安装
    install_python(app_handle, installer_path.to_str().unwrap().to_string(), version, install_path, window).await?;
    
    Ok(())
}
