use super::super::{
    logger,
    process_manager,
    types::{CleanResult, JetBrainsInstallation, JetBrainsResidueScanResult, ScannedPath},
};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;

const EXCLUDED_NOTE: &str = "仅清理所选产品对应版本的配置/缓存；不清理其他 IDE 或其他版本";

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
    if !fallback.is_empty() && fallback != "未知" {
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
fn derive_config_dir_name(installation: &JetBrainsInstallation) -> Option<String> {
    let prefix = config_folder_prefix(&installation.product_name)?;
    let version = extract_version_from_name(&installation.product_name, &installation.version);
    if version.is_empty() {
        None
    } else {
        Some(format!("{}{}", prefix, version))
    }
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
        $keys = @()
        $roots = @(
            'HKCU:\SOFTWARE\JetBrains',
            'HKCU:\SOFTWARE\JavaSoft\Prefs\jetbrains',
            'HKLM:\SOFTWARE\JetBrains',
            'HKLM:\SOFTWARE\WOW6432Node\JetBrains'
        )
        foreach ($root in $roots) {
            if (-not (Test-Path $root)) { continue }
            # 匹配含产品前缀+版本的子键
            Get-ChildItem $root -ErrorAction SilentlyContinue | Where-Object {
                $_.PSChildName -like "$prefix*" -or $_.PSChildName -like "*$ver*"
            } | ForEach-Object {
                $keys += $_.PSPath -replace '^Microsoft\.PowerShell\.Core\\Registry::', '' -replace '^HKEY_CURRENT_USER', 'HKCU' -replace '^HKEY_LOCAL_MACHINE', 'HKLM'
            }
            # 如果根键本身就是该产品的（如 HKCU:\SOFTWARE\JetBrains\PhpStorm2026.2）
            $rootName = Split-Path $root -Leaf
            if ($rootName -like "$prefix*$ver*") {
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
        $bases = @(
            [Environment]::GetFolderPath('CommonPrograms'),
            [Environment]::GetFolderPath('Programs')
        )
        $results = @()
        foreach ($base in $bases) {
            if (-not (Test-Path $base)) { continue }
            Get-ChildItem -Path $base -Recurse -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -like "*$prefix*" -or ($ver -and $_.Name -like "*$ver*") } |
                ForEach-Object { $results += $_.FullName }
        }
        $results | Sort-Object -Unique
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 30).await {
        Ok(out) => out.stdout.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

async fn delete_registry_key(key: &str) -> Result<(), String> {
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

    // 1. 删除配置目录
    for d in scan.config_dirs.iter().chain(scan.cache_dirs.iter()) {
        if !d.exists {
            continue;
        }
        match tokio::fs::remove_dir_all(&d.path).await {
            Ok(_) => {
                result.cleaned_items.push(format!("删除目录 [{}]: {}", d.category, d.path));
                logger::info(&app_handle, &format!("已删除: {}", d.path));
            }
            Err(e) => {
                result.errors.push(format!("无法删除 {}: {}", d.path, e));
                logger::warn(&app_handle, &format!("删除失败 {}: {}", d.path, e));
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
