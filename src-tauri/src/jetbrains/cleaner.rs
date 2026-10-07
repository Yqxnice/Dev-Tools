use super::super::{
    detector_base,
    logger,
    process_manager,
    types::{CleanResult, JetBrainsInstallation, JetBrainsResidueScanResult, ScannedPath},
};
use crate::delete_safety::{sanitize_segment, validate_deletable_dir, validate_registry_key, DeletionRules};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const EXCLUDED_NOTE: &str = "仅清理所选产品对应版本的配置/缓存；不清理其他 IDE 或其他版本";

/// 允许删除其**子键**的注册表根
const ALLOWED_REGISTRY_PREFIXES: &[&str] = &[
    r"HKCU\SOFTWARE\JetBrains",
    r"HKCU\SOFTWARE\JavaSoft\Prefs\jetbrains",
    r"HKLM\SOFTWARE\JetBrains",
    r"HKLM\SOFTWARE\WOW6432Node\JetBrains",
];

/// 允许**整键删除**的产品根键
const ALLOWED_REGISTRY_EXACT: &[&str] = &[r"HKCU\SOFTWARE\JavaSoft\Prefs\jetbrains"];

/// 产品显示名 -> 配置/缓存目录前缀映射（JetBrains 实际文件夹命名规律）
fn config_folder_prefix(product_name: &str) -> Option<String> {
    let n = product_name.to_lowercase();
    if n.contains("intellij idea") {
        Some("IntelliJIdea".into())
    } else if n.contains("pycharm") {
        Some("PyCharm".into())
    } else if n.contains("webstorm") {
        Some("WebStorm".into())
    } else if n.contains("goland") {
        Some("GoLand".into())
    } else if n.contains("clion") {
        Some("CLion".into())
    } else if n.contains("datagrip") {
        Some("DataGrip".into())
    } else if n.contains("phpstorm") {
        Some("PhpStorm".into())
    } else if n.contains("rubymine") {
        Some("RubyMine".into())
    } else if n.contains("rustrover") {
        Some("RustRover".into())
    } else if n.contains("rider") {
        Some("Rider".into())
    } else if n.contains("appcode") {
        Some("AppCode".into())
    } else {
        None
    }
}

/// 从 DisplayName 中提取版本号（如 "IntelliJ IDEA 2026.2" -> "2026.2"）
fn extract_version_from_name(name: &str, fallback: &str) -> String {
    // 优先使用 registry DisplayVersion
    if !fallback.is_empty() && fallback != detector_base::ARCH_UNKNOWN {
        return fallback.to_string();
    }
    // 从产品名末尾提取 x.y 形式的版本
    let re = regex::Regex::new(r"(\d+\.\d+(?:\.\d+)?)").unwrap();
    if let Some(cap) = re.captures(name) {
        return cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
    }
    String::new()
}

/// 推导该安装对应的配置/缓存目录名（如 IntelliJIdea2026.2）
///
/// 版本号来自前端 `installation.version`，必须作为**单个路径片段**净化：
/// 否则 `version = "..\..\..\"` 会让 `base.join(name)` 逃出
/// `%APPDATA%\JetBrains`，随后 `remove_dir_all` 可删任意目录。
fn derive_config_dir_name(installation: &JetBrainsInstallation) -> Option<String> {
    let prefix = config_folder_prefix(&installation.product_name)?;
    let version = extract_version_from_name(&installation.product_name, &installation.version);
    if version.is_empty() {
        None
    } else {
        match sanitize_segment(&format!("{}{}", prefix, version)) {
            Ok(name) => Some(name),
            Err(e) => {
                eprintln!("[WARN] 跳过非法配置目录名: {}", e);
                None
            }
        }
    }
}

/// 配置/缓存目录删除规则：必须位于 JetBrains 自身的配置/缓存根目录之下，
/// 且不得是根目录本身（`allowed_roots` 模式已内置该校验）。
fn jetbrains_dir_rules() -> DeletionRules {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(d) = dirs::config_dir() {
        roots.push(d.join("JetBrains"));
    }
    if let Some(d) = dirs::cache_dir() {
        roots.push(d.join("JetBrains"));
    }
    DeletionRules::new().allowed_roots(roots)
}

fn roaming_jetbrains_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("JetBrains"))
}

fn local_jetbrains_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("JetBrains"))
}

/// 扫描该安装对应版本的残留（配置目录、缓存目录、注册表、开始菜单快捷方式）
pub async fn scan_jetbrains_residuals(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> JetBrainsResidueScanResult {
    let label = format!("{} ({})", installation.product_name, installation.version);
    logger::info(&app_handle, &format!("开始扫描 JetBrains 残留: {}", label));

    let target_dir = derive_config_dir_name(&installation);
    let mut config_dirs = Vec::new();
    let mut cache_dirs = Vec::new();

    if let Some(dir_name) = &target_dir {
        // 配置目录：%APPDATA%\JetBrains\{dir}
        if let Some(base) = roaming_jetbrains_dir() {
            let path = base.join(dir_name);
            config_dirs.push(ScannedPath {
                path: path.to_string_lossy().to_string(),
                category: "config".to_string(),
                exists: path.exists(),
            });
        }
        // 缓存目录：%LOCALAPPDATA%\JetBrains\{dir}
        if let Some(base) = local_jetbrains_dir() {
            let path = base.join(dir_name);
            cache_dirs.push(ScannedPath {
                path: path.to_string_lossy().to_string(),
                category: "cache".to_string(),
                exists: path.exists(),
            });
        }
    } else {
        logger::warn(&app_handle, "无法推导配置目录名，跳过目录扫描");
    }

    let registry_keys = scan_registry_keys(&app_handle, &installation).await;
    let start_menu = scan_start_menu_shortcuts(&installation).await;

    logger::info(
        &app_handle,
        &format!(
            "扫描完成 [{}]: {} 个配置目录, {} 个缓存目录, {} 个注册表项, {} 个快捷方式",
            label,
            config_dirs.iter().filter(|d| d.exists).count(),
            cache_dirs.iter().filter(|d| d.exists).count(),
            registry_keys.len(),
            start_menu.len()
        ),
    );

    JetBrainsResidueScanResult {
        product_label: label,
        config_dirs,
        cache_dirs,
        registry_keys,
        start_menu_shortcuts: start_menu,
        excluded_note: EXCLUDED_NOTE.to_string(),
    }
}

async fn scan_registry_keys(app_handle: &AppHandle, installation: &JetBrainsInstallation) -> Vec<String> {
    let prefix = match config_folder_prefix(&installation.product_name) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let version = extract_version_from_name(&installation.product_name, &installation.version);

    let mut env_vars = HashMap::new();
    env_vars.insert("PREFIX".to_string(), prefix);
    env_vars.insert("VERSION".to_string(), version);

    let script = r#"
        $prefix = $env:PREFIX
        $ver = $env:VERSION
        if ([string]::IsNullOrWhiteSpace($prefix)) { exit 0 }
        # 只按「产品前缀[+版本]」匹配：
        # - 原实现 `-like "*$ver*"` 在 $ver 为空时恒真，会命中该根键下所有产品
        # - 即便 $ver 非空，`*2026.2*` 也会匹配到其他产品同版本的子键
        #   （如前缀 IntelliJIdea 却命中 PyCharm2026.2），造成跨产品误删
        $matchName = if ([string]::IsNullOrWhiteSpace($ver)) { "$prefix*" } else { "$prefix*$ver*" }
        $keys = @()
        $roots = @(
            'HKCU:\SOFTWARE\JetBrains',
            'HKCU:\SOFTWARE\JavaSoft\Prefs\jetbrains',
            'HKLM:\SOFTWARE\JetBrains',
            'HKLM:\SOFTWARE\WOW6432Node\JetBrains'
        )
        foreach ($root in $roots) {
            if (-not (Test-Path $root)) { continue }
            # 匹配含产品前缀（+版本）的子键
            Get-ChildItem $root -ErrorAction SilentlyContinue | Where-Object {
                $_.PSChildName -like $matchName
            } | ForEach-Object {
                $keys += $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::', '' -replace '^HKEY_CURRENT_USER', 'HKCU' -replace '^HKEY_LOCAL_MACHINE', 'HKLM'
            }
            # 如果根键本身就是该产品的（如 HKCU:\SOFTWARE\JetBrains\PhpStorm2026.2）
            $rootName = Split-Path $root -Leaf
            if ($rootName -like $matchName) {
                $keys += $root -replace '^HKCU:', 'HKCU' -replace '^HKLM:', 'HKLM'
            }
        }
        $keys | Sort-Object -Unique
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 30).await {
        Ok(out) => out.stdout.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect(),
        Err(e) => {
            logger::warn(app_handle, &format!("扫描注册表失败: {}", e));
            Vec::new()
        }
    }
}

async fn scan_start_menu_shortcuts(installation: &JetBrainsInstallation) -> Vec<String> {
    let prefix = match config_folder_prefix(&installation.product_name) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let version = extract_version_from_name(&installation.product_name, &installation.version);

    let mut env_vars = HashMap::new();
    env_vars.insert("PREFIX".to_string(), prefix);
    env_vars.insert("VERSION".to_string(), version);

    let script = r#"
        $prefix = $env:PREFIX
        $ver = $env:VERSION
        if ([string]::IsNullOrWhiteSpace($prefix)) { exit 0 }
        $hasVer = -not [string]::IsNullOrWhiteSpace($ver)
        $bases = @(
            [Environment]::GetFolderPath('CommonPrograms'),
            [Environment]::GetFolderPath('Programs')
        )
        $results = @()
        foreach ($base in $bases) {
            if (-not (Test-Path $base)) { continue }
            # 只取 JetBrains 供应商目录之下的项。
            # 原实现 `$_.Name -like "*$ver*"` 会命中开始菜单里**任何**名字含版本号
            # 的目录（其他产品的同版本目录也会被返回后整棵删除）
            Get-ChildItem -Path $base -Recurse -Directory -ErrorAction SilentlyContinue |
                Where-Object {
                    $_.FullName -like '*\JetBrains\*' -and
                    (-not $hasVer -or $_.FullName -like "*$ver*")
                } |
                ForEach-Object { $results += $_.FullName }
        }
        $results | Sort-Object -Unique
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 30).await {
        Ok(out) => out.stdout.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

/// 开始菜单删除白名单：必须位于 CommonPrograms/Programs 之下，
/// 且路径中必须出现 `JetBrains`（供应商目录），防止递归扫描误伤其他产品。
fn validate_start_menu_dir(raw: &str) -> Result<PathBuf, String> {
    if raw.contains("..") || raw.contains('*') || raw.contains('?') {
        return Err("路径含通配符或上级引用".to_string());
    }
    let canonical = Path::new(raw)
        .canonicalize()
        .map_err(|e| format!("无法解析路径: {}", e))?;
    if !canonical.is_dir() {
        return Err("目标不是目录".to_string());
    }

    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["APPDATA", "PROGRAMDATA"] {
        if let Ok(v) = std::env::var(var) {
            bases.push(
                PathBuf::from(v)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
    }
    if bases.is_empty() {
        return Err("无法定位开始菜单目录".to_string());
    }
    if !bases.iter().any(|b| canonical.starts_with(b)) {
        return Err("不在开始菜单目录内".to_string());
    }

    let lowered = canonical.to_string_lossy().to_lowercase();
    if !lowered.contains("jetbrains") {
        return Err("不在 JetBrains 供应商目录内".to_string());
    }

    // 不允许删掉 `Programs\JetBrains` 根本身
    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name.eq_ignore_ascii_case("JetBrains") {
        return Err("拒绝删除开始菜单 JetBrains 根目录".to_string());
    }
    Ok(canonical)
}

async fn delete_registry_key(key: &str) -> Result<(), String> {
    // 删除前白名单校验
    validate_registry_key(key, ALLOWED_REGISTRY_PREFIXES, ALLOWED_REGISTRY_EXACT)?;

    let reg_result = process_manager::execute_command("reg", &["delete", key, "/f"]).await;
    if let Ok(output) = reg_result {
        if output.exit_code == 0 {
            return Ok(());
        }
    }
    let normalized = if key.starts_with("HKLM") {
        key.replace("HKLM", "HKLM:")
    } else if key.starts_with("HKCU") {
        key.replace("HKCU", "HKCU:")
    } else {
        key.to_string()
    };
    let mut env_vars = HashMap::new();
    env_vars.insert("REG_KEY".to_string(), normalized);
    let script = r#"
        try {
            Remove-Item -Path $env:REG_KEY -Recurse -Force -ErrorAction Stop
            Write-Output 'SUCCESS'
        } catch {
            Write-Output "ERROR: $($_.Exception.Message)"
        }
    "#;
    let out = process_manager::execute_powershell_env(script, &env_vars, 30).await
        .map_err(|e| format!("PowerShell 删除失败: {}", e))?;
    if out.stdout.trim() == "SUCCESS" {
        Ok(())
    } else {
        Err(out.stdout.trim().to_string())
    }
}

/// 清理该安装对应版本的残留
pub async fn clean_jetbrains_residuals(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> CleanResult {
    let label = format!("{} ({})", installation.product_name, installation.version);
    let mut result = CleanResult {
        success: true,
        message: format!("{} 残留清理完成", label),
        cleaned_items: Vec::new(),
        errors: Vec::new(),
    };

    logger::info(&app_handle, &format!("开始清理 JetBrains 残留: {}", label));
    logger::info(&app_handle, EXCLUDED_NOTE);

    let scan = scan_jetbrains_residuals(app_handle.clone(), installation).await;
    let rules = jetbrains_dir_rules();

    // 1. 删除配置目录（必须通过受控基目录校验，防止逃出 %APPDATA%\JetBrains）
    for d in scan.config_dirs.iter().chain(scan.cache_dirs.iter()) {
        if !d.exists {
            continue;
        }
        let canonical = match validate_deletable_dir(&d.path, &rules) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("已阻止删除 [{}] {}: {}", d.category, d.path, e);
                logger::warn(&app_handle, &msg);
                result.errors.push(msg);
                continue;
            }
        };
        let target = canonical.to_string_lossy().to_string();
        match tokio::fs::remove_dir_all(&canonical).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除目录 [{}]: {}", d.category, target));
                logger::info(&app_handle, &format!("已删除: {}", target));
            }
            Err(e) => {
                result.errors.push(format!("无法删除 {}: {}", target, e));
                logger::warn(&app_handle, &format!("删除失败 {}: {}", target, e));
            }
        }
    }

    // 2. 删除注册表项
    for key in &scan.registry_keys {
        match delete_registry_key(key).await {
            Ok(_) => result.cleaned_items.push(format!("删除注册表项: {}", key)),
            Err(e) => {
                let lower = e.to_lowercase();
                if !lower.contains("找不到") && !lower.contains("不存在") && !lower.contains("not found") && !lower.contains("does not exist") {
                    result.errors.push(format!("删除注册表 {} 失败: {}", key, e));
                }
            }
        }
    }

    // 3. 删除开始菜单快捷方式
    for path in &scan.start_menu_shortcuts {
        if let Err(e) = validate_start_menu_dir(path) {
            let msg = format!("已阻止删除开始菜单路径 {}: {}", path, e);
            logger::warn(&app_handle, &msg);
            result.errors.push(msg);
            continue;
        }
        match tokio::fs::remove_dir_all(path).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除开始菜单快捷方式: {}", path));
                logger::info(&app_handle, &format!("已删除快捷方式: {}", path));
            }
            Err(e) => result.errors.push(format!("无法删除快捷方式 {}: {}", path, e)),
        }
    }

    if !result.errors.is_empty() {
        result.success = false;
        result.message = format!(
            "{} 部分清理失败（成功 {} 项，失败 {} 项）",
            label,
            result.cleaned_items.len(),
            result.errors.len()
        );
    } else if result.cleaned_items.is_empty() {
        result.message = format!("{} 未发现需要清理的残留", label);
    }

    result
}
