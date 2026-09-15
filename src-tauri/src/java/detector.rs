use super::super::{detector_base, logger, process_manager};
use super::super::types::JavaVersion;
use once_cell::sync::Lazy;
use regex::Regex;
use std::env;
use std::path::Path;
use tauri::AppHandle;

impl detector_base::RuntimeInstance for JavaVersion {
    fn get_executable(&self) -> &str { &self.executable }
    fn get_version(&self) -> &str { &self.version }
    fn get_path(&self) -> &str { &self.path }
    fn get_manager(&self) -> &str { &self.manager }
    fn get_status(&self) -> &str { &self.status }
    fn set_status(&mut self, s: String) { self.status = s; }
}

/// 匹配 `java -version` 输出中的版本号
/// 形如：`openjdk version "17.0.9"`、`java version "1.8.0_381"`、`openjdk version "21.0.5" 2023-10-17`
static VERSION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""(\d+(?:\.\d+)*(?:[._]\d+)?(?:\+\d+)?)"#).unwrap()
});

/// 从 `java -version` 输出解析版本号
/// 该命令输出在 stderr（Java 特有），调用方需传入合并后的输出
fn parse_version(output: &str) -> Option<String> {
    // 取第一行包含 "version" 的内容
    for line in output.lines() {
        if line.to_lowercase().contains("version") {
            if let Some(cap) = VERSION_REGEX.captures(line) {
                let raw = cap.get(1)?.as_str().to_string();
                return Some(normalize_version(&raw));
            }
        }
    }
    // 兜底：全文搜索
    VERSION_REGEX
        .captures(output)
        .map(|c| normalize_version(c.get(1).unwrap().as_str()))
}

/// 规范化版本号：`1.8.0_381` → `8.0.381`（去掉 Java 8 的 1. 前缀）
fn normalize_version(raw: &str) -> String {
    // Java 8 以下版本形如 1.8.0_381，去掉 1. 前缀并将 _ 转 .
    if let Some(rest) = raw.strip_prefix("1.8.") {
        // rest 形如 "0_381"，取掉前导 "0_" 得到 "381"
        let cleaned = if let Some(after_underscore) = rest.strip_prefix("0_") {
            after_underscore.to_string()
        } else {
            rest.replace('_', ".")
        };
        return format!("8.0.{}", cleaned);
    }
    raw.to_string()
}

/// 从 `java -version` 输出推断厂商
fn parse_vendor(output: &str) -> String {
    let lower = output.to_lowercase();
    if lower.contains("temurin") || lower.contains("adoptium") || lower.contains("adoptopenjdk") {
        return "Adoptium".to_string();
    }
    if lower.contains("microsoft") {
        return "Microsoft".to_string();
    }
    if lower.contains("zulu") {
        return "Zulu".to_string();
    }
    if lower.contains("amazon") || lower.contains("corretto") {
        return "Amazon".to_string();
    }
    if lower.contains("oracle") {
        return "Oracle".to_string();
    }
    // Oracle JDK 通常输出 "Java(TM) SE Runtime Environment" 或 "Java HotSpot(TM)"
    if lower.contains("java(tm) se") || lower.contains("hotspot(tm)") {
        return "Oracle".to_string();
    }
    if lower.contains("openjdk") {
        return "OpenJDK".to_string();
    }
    "Unknown".to_string()
}

/// 检查指定 java.exe 路径是否有效并返回版本信息
pub(crate) async fn check_java_at_path(
    path: &str,
    manager: &str,
) -> Option<JavaVersion> {
    let java_exe = Path::new(path);

    // 占位符通常很小
    if let Ok(meta) = java_exe.metadata() {
        if meta.len() < 1024 * 10 {
            return None;
        }
    }

    let result = process_manager::execute_command(path, &["-version"]).await;
    match result {
        Ok(output) if output.exit_code == 0 => {
            // java -version 输出到 stderr
            let combined = if output.stderr.is_empty() {
                output.stdout.clone()
            } else {
                output.stderr.clone()
            };
            let vendor = parse_vendor(&combined);
            parse_version(&combined).map(|version| {
                let parent_path = java_exe
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                JavaVersion {
                    version,
                    path: parent_path,
                    executable: path.to_string(),
                    vendor,
                    manager: manager.to_string(),
                    status: "已安装".to_string(),
                }
            })
        }
        _ => None,
    }
}

/// 检测默认 Java（PATH 中的 java）
pub async fn detect_default_java(_app_handle: AppHandle) -> Result<Option<JavaVersion>, String> {
    Ok(check_java_at_path("java", "system").await)
}

/// 从 JAVA_HOME 环境变量扫描
async fn scan_java_home(_app_handle: &AppHandle) -> Vec<JavaVersion> {
    let mut versions = Vec::new();

    let java_home = env::var("JAVA_HOME").ok().filter(|s| !s.is_empty());
    let Some(home) = java_home else { return versions; };

    let home_path = Path::new(&home);
    if !home_path.exists() || !home_path.is_dir() {
        return versions;
    }

    // JAVA_HOME 可能指向 JDK 根目录，也可能是 bin 目录
    let bin_dir = if home_path.file_name().and_then(|s| s.to_str()) == Some("bin") {
        home_path.to_path_buf()
    } else {
        home_path.join("bin")
    };

    let java_exe = bin_dir.join(if cfg!(windows) { "java.exe" } else { "java" });
    if java_exe.exists() {
        if let Some(p) = java_exe.to_str() {
            if let Some(v) = check_java_at_path(p, "JAVA_HOME").await {
                versions.push(v);
            }
        }
    }

    versions
}

/// 扫描系统常见安装位置
async fn scan_system_installs(app_handle: &AppHandle) -> Vec<JavaVersion> {
    let mut versions = Vec::new();
    let system_paths = vec![
        r"C:\Program Files\Java",
        r"C:\Program Files (x86)\Java",
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft\jdk",
        r"C:\Program Files\Amazon Corretto",
        r"C:\Program Files\Zulu",
    ];

    for base in system_paths {
        let base_path = Path::new(base);
        if !base_path.exists() || !base_path.is_dir() {
            continue;
        }
        match tokio::fs::read_dir(base_path).await {
            Ok(mut entries) => {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let java_exe = entry_path
                            .join("bin")
                            .join(if cfg!(windows) { "java.exe" } else { "java" });
                        if java_exe.exists() {
                            if let Some(p) = java_exe.to_str() {
                                if let Some(v) = check_java_at_path(p, "system").await {
                                    versions.push(v);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // 子目录扫描失败：跳过继续，不中断整体检测
                logger::warn(
                    app_handle,
                    &format!("扫描目录 {} 失败: {}", base, e),
                );
            }
        }
    }

    versions
}

/// 从 PATH 环境变量扫描
async fn scan_from_path(_app_handle: &AppHandle) -> Vec<JavaVersion> {
    let mut versions = Vec::new();

    if let Ok(path_env) = env::var("PATH") {
        for dir in path_env.split(';').filter(|s| !s.is_empty()) {
            let java_exe = Path::new(dir).join(if cfg!(windows) { "java.exe" } else { "java" });
            if java_exe.exists() {
                if let Some(p) = java_exe.to_str() {
                    if let Some(v) = check_java_at_path(p, "system").await {
                        versions.push(v);
                    }
                }
            }
        }
    }

    versions
}

pub async fn detect_java_versions(app_handle: AppHandle) -> Result<Vec<JavaVersion>, String> {
    logger::info(&app_handle, "开始检测 Java...");
    let mut versions = Vec::new();

    // 按优先级检测
    versions.extend(scan_java_home(&app_handle).await);
    versions.extend(scan_system_installs(&app_handle).await);
    versions.extend(scan_from_path(&app_handle).await);

    let versions = detector_base::dedupe_by_executable(versions);

    logger::info(
        &app_handle,
        &format!("检测完成，发现 {} 个 Java 版本", versions.len()),
    );
    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_openjdk_17() {
        let output = r#"openjdk version "17.0.9" 2023-10-17
OpenJDK Runtime Environment Temurin-17.0.9+9 (build 17.0.9+9)
OpenJDK 64-Bit Server VM Temurin-17.0.9+9 (build 17.0.9+9, mixed mode, sharing)"#;
        assert_eq!(parse_version(output), Some("17.0.9".to_string()));
    }

    #[test]
    fn parse_version_oracle_8() {
        let output = r#"java version "1.8.0_381"
Java(TM) SE Runtime Environment (build 1.8.0_381-b10)
Java HotSpot(TM) 64-Bit Server VM (build 25.381-b10, mixed mode)"#;
        assert_eq!(parse_version(output), Some("8.0.381".to_string()));
    }

    #[test]
    fn parse_version_adoptium_21() {
        let output = r#"openjdk version "21.0.5" 2023-10-17 LTS
OpenJDK Runtime Environment Temurin-21.0.5+11 (build 21.0.5+11-LTS)"#;
        assert_eq!(parse_version(output), Some("21.0.5".to_string()));
    }

    #[test]
    fn parse_vendor_temurin() {
        let output = "openjdk version \"17.0.9\" 2023-10-17\nOpenJDK Runtime Environment Temurin-17.0.9+9";
        assert_eq!(parse_vendor(output), "Adoptium");
    }

    #[test]
    fn parse_vendor_oracle() {
        let output = "java version \"1.8.0_381\"\nJava(TM) SE Runtime Environment";
        assert_eq!(parse_vendor(output), "Oracle");
    }

    #[test]
    fn normalize_java8_version() {
        assert_eq!(normalize_version("1.8.0_381"), "8.0.381");
        assert_eq!(normalize_version("17.0.9"), "17.0.9");
    }
}
