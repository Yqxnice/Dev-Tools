//! 环境变量检测：读取注册表 HKLM + HKCU 的 PATH 与常见 env 变量。

use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    KEY_READ, REG_VALUE_TYPE, REG_SZ,
};

use crate::types::{EnvPath, EnvVariable, PathConflict};

/// 用户级注册表 Environment 子键路径
const USER_ENV_SUBKEY: &str = "Environment";

/// 系统级注册表 Environment 子键路径
const SYSTEM_ENV_SUBKEY: &str =
    "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";

/// 把 &str 转 wide string（含 NUL 终止符）
fn to_wide_with_nul(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide
}

/// 打开注册表子键并读取 REG_SZ 值
///
/// 成功返回值字符串；失败返回 None（注册表项不存在/无值/类型不匹配）
fn read_reg_sz(hkey_root: HKEY, subkey: &str, value_name: &str) -> Option<String> {
    let subkey_w = to_wide_with_nul(subkey);
    let name_w = to_wide_with_nul(value_name);

    unsafe {
        let mut hkey: HKEY = std::mem::zeroed();
        let open_result = RegOpenKeyExW(
            hkey_root,
            PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );
        if open_result.is_err() {
            return None;
        }

        // 先查长度，再读数据
        let mut buf_len: u32 = 0;
        let mut value_type: REG_VALUE_TYPE = REG_VALUE_TYPE(0);
        let query_result = RegQueryValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut buf_len),
        );
        let _ = RegCloseKey(hkey);
        if query_result.is_err() || value_type != REG_SZ {
            return None;
        }
        if buf_len == 0 {
            return Some(String::new());
        }

        // 重新打开读取数据（close 之后句柄失效）
        let mut hkey2: HKEY = std::mem::zeroed();
        let open2 = RegOpenKeyExW(
            hkey_root,
            PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey2,
        );
        if open2.is_err() {
            return None;
        }

        let mut buf: Vec<u8> = vec![0u8; buf_len as usize];
        let read_result = RegQueryValueExW(
            hkey2,
            PCWSTR(name_w.as_ptr()),
            None,
            None,
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut buf_len),
        );
        let _ = RegCloseKey(hkey2);
        if read_result.is_err() {
            return None;
        }

        // REG_SZ 是 UTF-16 字符串；可能以 NUL 结尾，也可能不
        let u16s: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_ne_bytes([c[0], c[1]]))
            .collect();
        let end = u16s.iter().position(|&c| c == 0).unwrap_or(u16s.len());
        Some(String::from_utf16_lossy(&u16s[..end]))
    }
}

/// 读取注册表 PATH，按 `;` 切分为路径列表
fn read_path_list(hkey_root: HKEY, subkey: &str, scope: &str) -> Vec<EnvPath> {
    let raw = read_reg_sz(hkey_root, subkey, "Path").unwrap_or_default();
    raw.split(';')
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let trimmed = p.trim().to_string();
            let exists = std::path::Path::new(&trimmed).exists();
            let tool = detect_tool(&trimmed);
            EnvPath {
                path: trimmed,
                scope: scope.to_string(),
                exists,
                tool,
            }
        })
        .collect()
}

/// 从路径名识别属于哪个工具
///
/// 识别规则（保守，避免误判）：
/// - java: 路径包含 "java" 或 "jdk" (case-insensitive) 且以 \bin 结尾，或父目录名匹配
/// - python: 路径包含 "python"
/// - node: 路径包含 "nodejs" 或 "node" 且非 "node_modules"
/// - git: 路径包含 "git"
/// - mysql: 路径包含 "mysql"
/// - postgresql: 路径包含 "postgresql" 或 "pg"
pub fn detect_tool(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    if lower.contains("node_modules") {
        return None;
    }
    if lower.contains("\\java") || lower.contains("\\jdk") || lower.contains("/java") || lower.contains("/jdk") {
        return Some("java".to_string());
    }
    if lower.contains("python") {
        return Some("python".to_string());
    }
    if lower.contains("nodejs") || lower.contains("\\node") || lower.contains("/node") {
        return Some("node".to_string());
    }
    if lower.contains("\\git") || lower.contains("/git") {
        return Some("git".to_string());
    }
    if lower.contains("mysql") {
        return Some("mysql".to_string());
    }
    if lower.contains("postgresql") || lower.contains("\\pg") {
        return Some("postgresql".to_string());
    }
    None
}

/// 读取系统 + 用户 PATH 列表（系统在前，用户在后）
pub fn get_all_paths() -> Vec<EnvPath> {
    let mut paths = Vec::new();
    paths.extend(read_path_list(
        HKEY_LOCAL_MACHINE,
        SYSTEM_ENV_SUBKEY,
        "system",
    ));
    paths.extend(read_path_list(
        HKEY_CURRENT_USER,
        USER_ENV_SUBKEY,
        "user",
    ));
    paths
}

/// 读取常见环境变量
///
/// 列表选择：开发工具常用的 HOME / 路径变量
pub fn get_common_env_variables() -> Vec<EnvVariable> {
    const COMMON_VARS: &[&str] = &[
        "JAVA_HOME",
        "PYTHONHOME",
        "PYTHONPATH",
        "NODE_PATH",
        "M2_HOME",
        "MAVEN_HOME",
        "GRADLE_HOME",
        "GOPATH",
        "RUSTUP_HOME",
        "CARGO_HOME",
        "ANDROID_HOME",
        "ANT_HOME",
        "GIT_INSTALL_ROOT",
    ];

    let mut result = Vec::new();
    for &name in COMMON_VARS {
        // 优先用户级，回退系统级
        if let Some(v) = read_reg_sz(HKEY_CURRENT_USER, USER_ENV_SUBKEY, name) {
            result.push(EnvVariable {
                name: name.to_string(),
                value: v,
                scope: "user".to_string(),
            });
        } else if let Some(v) = read_reg_sz(HKEY_LOCAL_MACHINE, SYSTEM_ENV_SUBKEY, name) {
            result.push(EnvVariable {
                name: name.to_string(),
                value: v,
                scope: "system".to_string(),
            });
        }
    }
    result
}

/// 检测 PATH 冲突
///
/// 检测维度：
/// 1. duplicate：同一路径在 PATH 中多次出现（大小写不敏感）
/// 2. missing：路径在文件系统中不存在
/// 3. multi_version：同一工具识别出多个版本路径
pub fn detect_conflicts(paths: &[EnvPath]) -> Vec<PathConflict> {
    use std::collections::HashMap;

    let mut conflicts = Vec::new();

    // 1. 重复检测
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    for p in paths {
        let key = p.path.to_ascii_lowercase();
        let count = seen.entry(key).or_insert(0);
        *count += 1;
        if *count == 2 {
            duplicates.push(p.path.clone());
        }
    }
    if !duplicates.is_empty() {
        conflicts.push(PathConflict {
            kind: "duplicate".to_string(),
            tool: None,
            paths: duplicates.clone(),
            message: format!("检测到 {} 条重复路径", duplicates.len()),
        });
    }

    // 2. 不存在路径
    let missing: Vec<String> = paths
        .iter()
        .filter(|p| !p.exists)
        .map(|p| p.path.clone())
        .collect();
    if !missing.is_empty() {
        conflicts.push(PathConflict {
            kind: "missing".to_string(),
            tool: None,
            paths: missing.clone(),
            message: format!("检测到 {} 条不存在的路径", missing.len()),
        });
    }

    // 3. 多版本冲突：同一工具识别多个路径
    let mut tool_paths: HashMap<String, Vec<String>> = HashMap::new();
    for p in paths {
        if let Some(tool) = &p.tool {
            tool_paths
                .entry(tool.clone())
                .or_default()
                .push(p.path.clone());
        }
    }
    for (tool, tool_path_list) in tool_paths {
        if tool_path_list.len() >= 2 {
            conflicts.push(PathConflict {
                kind: "multi_version".to_string(),
                tool: Some(tool.clone()),
                paths: tool_path_list.clone(),
                message: format!(
                    "检测到 {} 个 {} 路径：{}",
                    tool_path_list.len(),
                    tool,
                    tool_path_list.join("；")
                ),
            });
        }
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_tool_recognizes_java() {
        assert_eq!(
            detect_tool(r"C:\Program Files\Java\jdk-21\bin"),
            Some("java".to_string())
        );
        assert_eq!(
            detect_tool(r"C:\Java\jdk-17\bin"),
            Some("java".to_string())
        );
    }

    #[test]
    fn detect_tool_recognizes_python() {
        assert_eq!(
            detect_tool(r"C:\Python312\Scripts"),
            Some("python".to_string())
        );
        assert_eq!(
            detect_tool(r"C:\Users\xxx\AppData\Local\Programs\Python\Python312"),
            Some("python".to_string())
        );
    }

    #[test]
    fn detect_tool_recognizes_node() {
        assert_eq!(
            detect_tool(r"C:\Program Files\nodejs"),
            Some("node".to_string())
        );
    }

    #[test]
    fn detect_tool_skips_node_modules() {
        assert_eq!(
            detect_tool(r"C:\project\node_modules\.bin"),
            None
        );
    }

    #[test]
    fn detect_tool_recognizes_git() {
        assert_eq!(
            detect_tool(r"C:\Program Files\Git\cmd"),
            Some("git".to_string())
        );
    }

    #[test]
    fn detect_tool_returns_none_for_unknown() {
        assert_eq!(detect_tool(r"C:\Windows\system32"), None);
        assert_eq!(detect_tool(r"C:\Windows"), None);
    }

    #[test]
    fn detect_conflicts_finds_duplicates_case_insensitive() {
        let paths = vec![
            EnvPath { path: r"C:\Python312\Scripts".to_string(), scope: "system".to_string(), exists: true, tool: Some("python".to_string()) },
            EnvPath { path: r"c:\python312\scripts".to_string(), scope: "user".to_string(), exists: true, tool: Some("python".to_string()) },
        ];
        let conflicts = detect_conflicts(&paths);
        assert_eq!(conflicts.len(), 2); // duplicate + multi_version
        let dup = conflicts.iter().find(|c| c.kind == "duplicate").unwrap();
        assert_eq!(dup.paths.len(), 1);
    }

    #[test]
    fn detect_conflicts_finds_missing_paths() {
        let paths = vec![
            EnvPath { path: r"C:\nonexistent\path\xyz".to_string(), scope: "system".to_string(), exists: false, tool: None },
        ];
        let conflicts = detect_conflicts(&paths);
        let missing = conflicts.iter().find(|c| c.kind == "missing").unwrap();
        assert_eq!(missing.paths.len(), 1);
    }

    #[test]
    fn detect_conflicts_finds_multi_version() {
        let paths = vec![
            EnvPath { path: r"C:\Java\jdk-17\bin".to_string(), scope: "system".to_string(), exists: true, tool: Some("java".to_string()) },
            EnvPath { path: r"C:\Java\jdk-21\bin".to_string(), scope: "system".to_string(), exists: true, tool: Some("java".to_string()) },
        ];
        let conflicts = detect_conflicts(&paths);
        let mv = conflicts.iter().find(|c| c.kind == "multi_version").unwrap();
        assert_eq!(mv.tool.as_deref(), Some("java"));
        assert_eq!(mv.paths.len(), 2);
    }

    #[test]
    fn detect_conflicts_empty_for_clean_paths() {
        let paths = vec![
            EnvPath { path: r"C:\Windows\system32".to_string(), scope: "system".to_string(), exists: true, tool: None },
        ];
        let conflicts = detect_conflicts(&paths);
        assert!(conflicts.is_empty());
    }
}
