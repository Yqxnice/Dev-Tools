//! 运行时默认版本切换（Feature 4）
//!
//! 设计：通过修改用户级环境变量（HKCU\Environment）让指定版本成为系统"默认"：
//!  - java:  写 JAVA_HOME = version_path（PATH 中已有 %JAVA_HOME%\bin 即可生效）
//!  - python: 写 PYTHONHOME = version_path（不影响 PATH，仅作为 Python 启动时的家目录）
//!  - node:  无 HOME 概念；返回提示让用户手动调整 PATH 优先级
//!
//! 安全约束：
//!  - 仅写用户级注册表（HKCU），不需要管理员权限
//!  - 写入后广播 WM_SETTINGCHANGE 让新启动的进程感知；已运行进程不受影响
//!  - 不修改 PATH（避免破坏其他工具）
//!  - 失败时返回错误字符串，不部分写入

use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, HKEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG,
};

/// 用户环境变量注册表子键路径
const ENV_SUBKEY: &str = "Environment";

/// 把 &str 转 wide string（含 NUL 终止符）
fn to_wide_with_nul(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0); // NUL terminator
    wide
}

/// 写入用户级环境变量（REG_SZ）
fn set_user_env_var(name: &str, value: &str) -> Result<(), String> {
    let subkey_w = to_wide_with_nul(ENV_SUBKEY);
    let name_w = to_wide_with_nul(name);
    // REG_SZ 数据需以 NUL 结尾的 wide string
    let mut value_w: Vec<u16> = value.encode_utf16().collect();
    value_w.push(0);
    let value_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(value_w.as_ptr() as *const u8, value_w.len() * 2)
    };

    unsafe {
        let mut hkey: HKEY = std::mem::zeroed();
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if open_result.is_err() {
            return Err(format!("打开注册表 HKCU\\Environment 失败: 错误码 {}", open_result.0));
        }
        let result = RegSetValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            0,
            REG_SZ,
            Some(value_bytes),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() {
            return Err(format!("写入注册表值失败: 错误码 {}", result.0));
        }
    }
    Ok(())
}

/// 广播 WM_SETTINGCHANGE，通知新进程环境变量已更新
fn broadcast_env_change() {
    unsafe {
        // lParam 指向字符串 "Environment"（UTF-16）
        let env_str = to_wide_with_nul("Environment");
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(env_str.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }
}

/// 设置运行时默认版本
///
/// - tool: 'java' | 'python' | 'node'（其他值返回错误）
/// - version_path: 该版本的安装路径（如 java 的 java.exe 父目录或 JAVA_HOME 路径）
///
/// 返回：成功为空字符串，失败为错误消息
pub fn set_default_runtime(tool: &str, version_path: &str) -> Result<String, String> {
    if version_path.trim().is_empty() {
        return Err("版本路径不能为空".to_string());
    }

    let env_name = match tool {
        "java" => "JAVA_HOME",
        "python" => "PYTHONHOME",
        "node" => {
            // Node 无 HOME 概念，仅返回提示
            return Err(
                "Node.js 没有统一的默认版本切换机制，请手动调整 PATH 优先级或使用 nvm-windows"
                    .to_string(),
            );
        }
        other => return Err(format!("不支持的工具类型: {}", other)),
    };

    set_user_env_var(env_name, version_path)?;
    broadcast_env_change();
    Ok(format!("已设置 {}={}（仅影响新启动的进程）", env_name, version_path))
}

/// Tauri 命令封装：写注册表 + 广播 + 返回成功消息
#[tauri::command]
pub fn set_default_runtime_command(
    tool: String,
    version_path: String,
) -> Result<String, String> {
    set_default_runtime(&tool, &version_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_path() {
        let r = set_default_runtime("java", "   ");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("版本路径不能为空"));
    }

    #[test]
    fn rejects_unknown_tool() {
        let r = set_default_runtime("rust", "/some/path");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("不支持的工具类型"));
    }

    #[test]
    fn node_returns_hint_message() {
        let r = set_default_runtime("node", "C:\\node\\v20");
        assert!(r.is_err());
        let msg = r.unwrap_err();
        assert!(msg.contains("nvm-windows"));
    }

    #[test]
    fn to_wide_with_nul_appends_zero() {
        let w = to_wide_with_nul("hi");
        assert_eq!(w, vec!['h' as u16, 'i' as u16, 0]);
    }

    #[test]
    fn to_wide_with_nul_handles_empty_string() {
        let w = to_wide_with_nul("");
        assert_eq!(w, vec![0]);
    }

    #[test]
    fn to_wide_with_nul_handles_non_ascii() {
        let w = to_wide_with_nul("中");
        assert_eq!(w.len(), 2); // 一个 BMP 字符 + NUL
        assert_eq!(w[1], 0);
    }
}
