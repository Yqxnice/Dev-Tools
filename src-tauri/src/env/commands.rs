//! 环境变量管理器 IPC 命令（Feature 3）

use crate::download_control::devtools_env_backup_dir;
use crate::types::{EnvPath, EnvVariable, PathConflict};
use super::detector;
use serde::Serialize;

/// 读取系统 + 用户 PATH 列表
#[tauri::command]
pub fn get_env_paths() -> Vec<EnvPath> {
    detector::get_all_paths()
}

/// 读取常见环境变量（JAVA_HOME/PYTHONHOME 等）
#[tauri::command]
pub fn get_env_variables() -> Vec<EnvVariable> {
    detector::get_common_env_variables()
}

/// 检测 PATH 冲突
#[tauri::command]
pub fn detect_path_conflicts(paths: Vec<EnvPath>) -> Vec<PathConflict> {
    detector::detect_conflicts(&paths)
}

/// 判断目标路径是否出现在给定的 PATH 列表中（大小写不敏感）
///
/// 抽取为独立纯函数便于单元测试；不读取注册表，依赖外部传入的 PATH 列表。
pub fn is_path_in_env_paths(target: &str, all_paths: &[EnvPath]) -> bool {
    let target_lower = target.to_ascii_lowercase();
    all_paths
        .iter()
        .any(|ep| ep.path.to_ascii_lowercase() == target_lower)
}

/// 在资源管理器中打开 PATH 条目
///
/// 与全局 `open_in_folder` 不同，本命令面向用户 PATH 环境变量中的任意路径
/// （如 `C:\Program Files\Java\jdk-17\bin`），不受 DevTools 自身目录白名单限制。
///
/// 纵深防御：path 必须真实出现在当前用户的系统/用户 PATH 注册表中，
/// 防止前端绕过 PATH 显示直接调用此命令打开任意敏感目录。
#[tauri::command]
pub fn open_path_entry(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 校验：路径必须真实出现在注册表 PATH 中
    let all_paths = detector::get_all_paths();
    if !is_path_in_env_paths(&path, &all_paths) {
        return Err("该路径不在 PATH 环境变量中".to_string());
    }

    // 若是文件，打开其父目录；若是目录，直接打开
    let open_target = if p.is_file() {
        p.parent().map(|parent| parent.to_path_buf()).unwrap_or(p.clone())
    } else {
        p.clone()
    };

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg(&open_target)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&open_target).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&open_target).spawn();
    }
    Ok(())
}

/// 打开 Windows 自带的"环境变量"编辑器
///
/// 调用 `rundll32.exe sysdm.cpl,EditEnvironmentVariables` 直接打开
/// 系统"环境变量"对话框（无需先经过"系统属性"高级页）。
/// 非 Windows 平台返回不支持错误。
#[tauri::command]
pub fn open_env_editor() -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .args(["sysdm.cpl,EditEnvironmentVariables"])
            .spawn()
            .map_err(|e| format!("启动环境变量编辑器失败: {}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = ();
        Err("当前系统不支持此功能".to_string())
    }
}

/// 环境变量备份文件结构
#[derive(Serialize)]
struct EnvBackup {
    /// 备份时间（Unix 秒）
    timestamp: u64,
    /// 备份时间（本地可读字符串，ISO 8601）
    datetime: String,
    /// 系统 + 用户 PATH 列表
    paths: Vec<EnvPath>,
    /// 常见环境变量
    variables: Vec<EnvVariable>,
}

/// 备份当前环境变量（PATH + 常见变量）到 JSON 文件
///
/// 文件保存到「下载/DevTools/env-backups/env-backup-{unix_secs}.json」，
/// 写入完成后在资源管理器中打开备份目录，返回文件完整路径。
#[tauri::command]
pub fn backup_env_variables() -> Result<String, String> {
    let paths = detector::get_all_paths();
    let variables = detector::get_common_env_variables();

    let unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let datetime = {
        // 简单格式化为本地时间 ISO 8601 近似字符串
        let secs = unix_secs;
        let days = secs / 86400;
        let remainder = secs % 86400;
        let hour = remainder / 3600;
        let minute = (remainder % 3600) / 60;
        let second = remainder % 60;
        // 自 1970-01-01 起的天数粗算为日期（仅用于文件名可读性，不保证时区正确）
        format!(
            "epoch+{}d {:02}:{:02}:{:02}",
            days, hour, minute, second
        )
    };

    let backup = EnvBackup {
        timestamp: unix_secs,
        datetime,
        paths,
        variables,
    };

    let json = serde_json::to_string_pretty(&backup)
        .map_err(|e| format!("序列化备份内容失败: {}", e))?;

    let base_dir = devtools_env_backup_dir();
    std::fs::create_dir_all(&base_dir)
        .map_err(|e| format!("创建备份目录失败: {}", e))?;

    let file_path = base_dir.join(format!("env-backup-{}.json", unix_secs));

    std::fs::write(&file_path, &json)
        .map_err(|e| format!("写入备份文件失败: {}", e))?;

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg(&base_dir)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&base_dir).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&base_dir).spawn();
    }

    Ok(file_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_path(path: &str, scope: &str) -> EnvPath {
        EnvPath {
            path: path.to_string(),
            scope: scope.to_string(),
            exists: true,
            tool: None,
        }
    }

    #[test]
    fn is_path_in_env_paths_matches_case_insensitive() {
        let paths = vec![
            make_path(r"C:\Python312\Scripts", "system"),
            make_path(r"C:\Program Files\Java\jdk-17\bin", "system"),
        ];
        assert!(is_path_in_env_paths(r"c:\python312\scripts", &paths));
        assert!(is_path_in_env_paths(
            r"C:\PROGRAM FILES\JAVA\JDK-17\bin",
            &paths
        ));
    }

    #[test]
    fn is_path_in_env_paths_rejects_unknown_path() {
        let paths = vec![make_path(r"C:\Python312\Scripts", "system")];
        assert!(!is_path_in_env_paths(r"C:\Windows\system32", &paths));
        assert!(!is_path_in_env_paths(r"C:\Python312", &paths)); // 父目录不算
    }

    #[test]
    fn is_path_in_env_paths_empty_list_returns_false() {
        assert!(!is_path_in_env_paths(r"C:\any\path", &[]));
    }
}
