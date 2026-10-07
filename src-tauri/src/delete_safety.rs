//! 删除操作安全校验：所有 `remove_dir_all` 前必须经过本模块。
//!
//! 背景：残留清理的路径大多由前端 IPC 传入（`instance.path` / `instance.data_dir`
//! / 配置目录名拼接），若直接交给 `remove_dir_all` 等于把任意目录删除权交给前端。
//!
//! 防线（逐层，任一不过即拒绝）：
//! 1. `sanitize_segment` —— 单个路径片段拒绝 `..` `/` `\` 通配符等穿越字符
//! 2. `contains_banned_token` —— 原始串拒绝 `..`、通配符、换行、NUL
//! 3. `canonicalize` —— 必须真实存在；消除 `..`、符号链接、8.3 短名
//! 4. `is_critical_system_path` —— 盘符根 / Windows / Program Files / 用户目录等**无条件拒绝**
//! 5. `DeletionRules` —— 产品关键词 + 最小深度 + 可选受控基目录，证明"这确实是要删的那类目录"
//!
//! 与 `lib.rs::is_path_allowed`（白名单，用于"打开目录"）互补：本模块是
//! "黑名单 + 归属证明"，因为合法安装位置可能在任意盘符，无法穷举白名单。

use std::path::{Component, Path, PathBuf};

/// 删除规则：调用方按产品特性声明，全部满足才允许删除。
#[derive(Debug, Clone, Default)]
pub struct DeletionRules {
    /// 路径中必须出现（忽略大小写）的关键词之一 —— 归属证明
    pub keywords: Vec<String>,
    /// 路径中必须出现（忽略大小写）的全部子串 —— 用于锁定到具体实例
    pub required_substrings: Vec<String>,
    /// 受控基目录：canonicalize 后必须位于其中至少一个之下
    pub allowed_roots: Vec<PathBuf>,
    /// 是否为"受控基目录"模式。由 `allowed_roots()` 设置，与
    /// `allowed_roots` 是否为空无关——这样即便运行时拿不到基目录
    /// （`dirs::config_dir()` 返回 None），也会**失败关闭**而不是退回关键词模式。
    pub root_restricted: bool,
    /// 最小路径组件数（含盘符），如 `C:\Program Files\MySQL` = 3
    pub min_components: usize,
}

impl DeletionRules {
    pub fn new() -> Self {
        Self {
            min_components: 3,
            ..Default::default()
        }
    }

    pub fn keywords(mut self, k: &[&str]) -> Self {
        self.keywords = k.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn required_substrings(mut self, s: &[&str]) -> Self {
        self.required_substrings = s.iter().map(|x| x.to_string()).collect();
        self
    }

    pub fn allowed_roots(mut self, r: Vec<PathBuf>) -> Self {
        self.allowed_roots = r;
        self.root_restricted = true;
        self
    }

    pub fn min_components(mut self, n: usize) -> Self {
        self.min_components = n;
        self
    }
}

/// 校验单个路径片段（用于 `base.join(segment)`），拒绝路径穿越与通配符。
///
/// 受控基目录（如 `%APPDATA%\JetBrains`）本身已可信，但拼接进去的
/// 版本号 / 目录名来自前端，必须保证它不会把结果带出基目录。
pub fn sanitize_segment(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("路径片段为空".to_string());
    }
    if trimmed.len() > 128 {
        return Err(format!("路径片段过长（{} 字节）", trimmed.len()));
    }
    if trimmed.contains("..") {
        return Err(format!("路径片段含上级目录引用: {}", trimmed));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains(':') {
        return Err(format!("路径片段含路径分隔符: {}", trimmed));
    }
    if trimmed.contains('*') || trimmed.contains('?') || trimmed.contains('[') {
        return Err(format!("路径片段含通配符: {}", trimmed));
    }
    if trimmed.contains('\0') || trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("路径片段含非法控制字符".to_string());
    }
    // Windows 保留名（CON/PRN/AUX/NUL/COM1..9/LPT1..9）与结尾点/空格
    let stem = trimmed.split('.').next().unwrap_or(trimmed).to_uppercase();
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        return Err(format!("路径片段为 Windows 保留名: {}", trimmed));
    }
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err(format!("路径片段以点或空格结尾: {}", trimmed));
    }
    Ok(trimmed.to_string())
}

/// 系统关键目录黑名单：**无论关键词校验是否通过，一律拒绝**。
///
/// 覆盖"盘符根 / Windows / Program Files / 用户目录 / 应用自身"等
/// 一旦删除即不可恢复的路径。
fn is_critical_system_path(canonical: &Path) -> bool {
    let mut critical: Vec<PathBuf> = Vec::new();

    // 各盘符根：C:\ D:\ ...
    if let Some(Component::Prefix(p)) = canonical.components().next() {
        critical.push(PathBuf::from(p.as_os_str()));
    }

    // 环境变量指向的系统/用户根目录
    for var in [
        "WINDIR",
        "SystemRoot",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "PROGRAMDATA",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PUBLIC",
        "ALLUSERSPROFILE",
    ] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                critical.push(PathBuf::from(v));
            }
        }
    }

    // 应用自身所在目录（防止删掉正在运行的程序）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            critical.push(parent.to_path_buf());
        }
    }

    critical.iter().any(|c| {
        match (c.canonicalize(), Some(canonical)) {
            (Ok(cc), Some(target)) => cc == target,
            _ => false,
        }
    })
}

/// 判断 `child` 是否位于 `base` 之下（含等于 base 本身），双方 canonicalize。
fn is_within(child: &Path, base: &Path) -> bool {
    match (child.canonicalize(), base.canonicalize()) {
        (Ok(c), Ok(b)) => c.starts_with(b),
        _ => false,
    }
}

/// 原始字符串层面的危险 token 检查（canonicalize 之前的粗筛）。
fn contains_banned_token(raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        return Some("路径为空".to_string());
    }
    if raw.contains("..") {
        return Some("路径含上级目录引用 (..)".to_string());
    }
    if raw.contains('*') || raw.contains('?') {
        return Some("路径含通配符".to_string());
    }
    if raw.contains('\0') || raw.contains('\n') || raw.contains('\r') {
        return Some("路径含非法控制字符".to_string());
    }
    None
}

/// 校验一个目录是否允许删除，通过则返回 canonicalize 后的路径。
///
/// 返回的是解析后的真实路径，调用方**必须使用返回值执行删除**，
/// 而不是删除原始传入的字符串（否则校验与执行可能作用于不同对象）。
pub fn validate_deletable_dir(raw: &str, rules: &DeletionRules) -> Result<PathBuf, String> {
    if let Some(reason) = contains_banned_token(raw) {
        return Err(format!("拒绝删除 {}: {}", raw, reason));
    }

    let canonical = Path::new(raw)
        .canonicalize()
        .map_err(|e| format!("路径无法解析 ({}): {}", raw, e))?;

    if !canonical.is_dir() {
        return Err(format!("目标不是目录: {}", canonical.display()));
    }

    if is_critical_system_path(&canonical) {
        return Err(format!("拒绝删除系统关键目录: {}", canonical.display()));
    }

    // 最小深度：组件数（`C:\a\b` = 3）
    let components = canonical.components().count();
    if components < rules.min_components {
        return Err(format!(
            "路径层级过浅（{} 层，要求至少 {} 层），拒绝删除: {}",
            components,
            rules.min_components,
            canonical.display()
        ));
    }

    // 受控基目录模式：必须落在其中之一之下
    if rules.root_restricted {
        if rules.allowed_roots.is_empty() {
            // 失败关闭：拿不到基目录时宁可拒绝删除，也不能退回关键词模式放行
            return Err("无法定位允许删除的基目录，已拒绝删除".to_string());
        }
        let ok = rules
            .allowed_roots
            .iter()
            .any(|root| is_within(&canonical, root));
        if !ok {
            return Err(format!(
                "路径不在允许删除的基目录内: {}",
                canonical.display()
            ));
        }
        // 受控模式下不允许删掉基目录本身
        if rules
            .allowed_roots
            .iter()
            .any(|r| r.canonicalize().map(|rc| rc == canonical).unwrap_or(false))
        {
            return Err(format!("拒绝删除基目录本身: {}", canonical.display()));
        }
        return Ok(canonical);
    }

    // 关键词模式：路径必须能证明属于目标产品
    let path_lower = canonical.to_string_lossy().to_lowercase();
    if !rules.keywords.is_empty() {
        let hit = rules.keywords.iter().any(|k| path_lower.contains(&k.to_lowercase()));
        if !hit {
            return Err(format!(
                "路径不含产品标识 {}，无法确认归属，拒绝删除: {}",
                rules.keywords.join("/"),
                canonical.display()
            ));
        }
    }
    for sub in &rules.required_substrings {
        if !path_lower.contains(&sub.to_lowercase()) {
            return Err(format!(
                "路径不含实例标识 \"{}\"，可能误删其他版本，拒绝删除: {}",
                sub,
                canonical.display()
            ));
        }
    }

    Ok(canonical)
}

/// 校验注册表键是否允许删除。
///
/// 与目录白名单互补：目录无法穷举白名单（安装位置任意），但注册表键的
/// 合法删除范围是**可枚举的**（本产品相关的几个根键之下），因此这里用
/// 白名单 + 深度约束，拒绝删除白名单之外的任何键。
///
/// * `key` —— 形如 `HKLM\SOFTWARE\MySQL AB\MySQL Server 8.0`
/// * `allowed_prefixes` —— 允许删除其**子键**的根（键必须严格位于其下）
/// * `allowed_exact` —— 允许**整键删除**的键（如产品自身的用户配置根键）
///
/// 两者分开是为了避免为了放行 `HKCU\SOFTWARE\PostgreSQL` 而把整个
/// `HKCU\SOFTWARE` 纳入可删前缀。
pub fn validate_registry_key(
    key: &str,
    allowed_prefixes: &[&str],
    allowed_exact: &[&str],
) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("注册表键为空".to_string());
    }
    if trimmed.contains("..") || trimmed.contains('*') || trimmed.contains('?') {
        return Err(format!("注册表键含通配符或上级引用: {}", trimmed));
    }
    if trimmed.contains('\0') || trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("注册表键含非法控制字符".to_string());
    }

    let normalized = trimmed
        .replace("HKEY_LOCAL_MACHINE", "HKLM")
        .replace("HKEY_CURRENT_USER", "HKCU");
    let lower = normalized.to_lowercase();

    // 精确整键删除
    if allowed_exact
        .iter()
        .any(|p| p.to_lowercase() == lower)
    {
        return Ok(());
    }

    // 前缀作用域：必须严格位于其下（前缀本身也不可删）
    let matched = allowed_prefixes
        .iter()
        .find(|p| lower.starts_with(&p.to_lowercase()));
    let prefix = match matched {
        Some(p) => *p,
        None => {
            return Err(format!("注册表键不在允许删除的根键范围内: {}", trimmed));
        }
    };

    if normalized.len() <= prefix.len() {
        return Err(format!("拒绝删除注册表根键: {}", trimmed));
    }
    // 分隔符必须紧随前缀之后（防止 `HKLM\SOFTWARE\MySQL ABX` 命中 `...MySQL AB`）
    if normalized.as_bytes().get(prefix.len()).copied() != Some(b'\\') {
        return Err(format!("注册表键前缀不匹配: {}", trimmed));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "devtools_del_safe_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("MySQL").join("MySQL Server 8.0")).unwrap();
        root
    }

    fn teardown(root: &Path) {
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sanitize_segment_accepts_plain_name() {
        assert_eq!(sanitize_segment("IntelliJIdea2026.2").unwrap(), "IntelliJIdea2026.2");
    }

    #[test]
    fn sanitize_segment_rejects_traversal() {
        assert!(sanitize_segment(r"..\..\Windows").is_err());
        assert!(sanitize_segment("a/b").is_err());
        assert!(sanitize_segment(r"a\b").is_err());
        assert!(sanitize_segment(r"C:\x").is_err());
    }

    #[test]
    fn sanitize_segment_rejects_wildcards_and_empty() {
        assert!(sanitize_segment("*").is_err());
        assert!(sanitize_segment("").is_err());
        assert!(sanitize_segment("   ").is_err());
        assert!(sanitize_segment("a\nb").is_err());
    }

    #[test]
    fn sanitize_segment_rejects_reserved_names_and_trailing_dot() {
        assert!(sanitize_segment("CON").is_err());
        assert!(sanitize_segment("nul.txt").is_err());
        assert!(sanitize_segment("name.").is_err());
    }

    #[test]
    fn banned_token_detected_before_canonicalize() {
        assert!(contains_banned_token(r"C:\a\..\b").is_some());
        assert!(contains_banned_token("").is_some());
        assert!(contains_banned_token(r"C:\mysql\*").is_some());
        assert!(contains_banned_token(r"C:\mysql").is_none());
    }

    #[test]
    fn rejects_path_with_traversal() {
        let rules = DeletionRules::new().keywords(&["mysql"]);
        assert!(validate_deletable_dir(r"C:\mysql\..\Windows", &rules).is_err());
    }

    #[test]
    fn rejects_nonexistent_path() {
        let rules = DeletionRules::new().keywords(&["mysql"]);
        assert!(validate_deletable_dir(r"C:\definitely_not_here_zzz", &rules).is_err());
    }

    #[test]
    fn rejects_missing_product_keyword() {
        let root = temp_tree("keyword");
        let target = root.join("NotOurProduct").join("Server");
        fs::create_dir_all(&target).unwrap();
        let rules = DeletionRules::new().keywords(&["mysql"]);
        let raw = target.to_string_lossy().to_string();
        assert!(validate_deletable_dir(&raw, &rules).is_err());
        teardown(&root);
    }

    #[test]
    fn rejects_when_instance_substring_missing() {
        let root = temp_tree("substring");
        let target = root.join("MySQL").join("MySQL Server 8.0");
        let raw = target.to_string_lossy().to_string();
        let rules = DeletionRules::new()
            .keywords(&["mysql"])
            .required_substrings(&["MySQL Server 5.7"]);
        assert!(validate_deletable_dir(&raw, &rules).is_err());
        teardown(&root);
    }

    #[test]
    fn accepts_matching_product_dir() {
        let root = temp_tree("accept");
        let target = root.join("MySQL").join("MySQL Server 8.0");
        let raw = target.to_string_lossy().to_string();
        let rules = DeletionRules::new()
            .keywords(&["mysql"])
            .required_substrings(&["MySQL Server 8.0"]);
        let out = validate_deletable_dir(&raw, &rules).unwrap();
        assert_eq!(
            out.canonicalize().unwrap(),
            target.canonicalize().unwrap()
        );
        teardown(&root);
    }

    #[test]
    fn rejects_shallow_path() {
        let root = temp_tree("shallow");
        // temp 根 = /devtools_del_safe_{pid}_shallow  -> 组件数可能较少
        let rules = DeletionRules::new()
            .keywords(&["mysql"])
            .min_components(99);
        let target = root.join("MySQL").join("MySQL Server 8.0");
        let raw = target.to_string_lossy().to_string();
        assert!(validate_deletable_dir(&raw, &rules).is_err());
        teardown(&root);
    }

    #[test]
    fn allowed_roots_mode_requires_base_membership() {
        let root = temp_tree("roots");
        let jetbrains = root.join("JetBrains");
        let cfg = jetbrains.join("IntelliJIdea2026.2");
        fs::create_dir_all(&cfg).unwrap();
        let outside = root.join("MySQL").join("MySQL Server 8.0");

        let rules = DeletionRules::new().allowed_roots(vec![jetbrains.clone()]);

        let inside_raw = cfg.to_string_lossy().to_string();
        assert!(validate_deletable_dir(&inside_raw, &rules).is_ok());

        let outside_raw = outside.to_string_lossy().to_string();
        assert!(validate_deletable_dir(&outside_raw, &rules).is_err());

        // 基目录本身不可删
        let base_raw = jetbrains.to_string_lossy().to_string();
        assert!(validate_deletable_dir(&base_raw, &rules).is_err());
        teardown(&root);
    }

    #[test]
    fn rejects_drive_root_unconditionally() {
        let rules = DeletionRules::new().keywords(&["c:"]); // 即便关键词"命中"也应被黑名单拦截
        // C:\ 是盘符根，canonicalize 后等于 critical 条目
        let out = validate_deletable_dir(r"C:\", &rules);
        // 系统盘符根必然触发深度或黑名单校验
        assert!(out.is_err());
    }
}
