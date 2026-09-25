use super::super::{logger, process_manager, types::JetBrainsInstallation};
use std::sync::LazyLock;
use regex::Regex;
use tauri::AppHandle;

/// 已安装 JetBrains 产品（用于版本检测与卸载清理的产品代号推断）
static INSTALLED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"IntelliJ IDEA|PyCharm|WebStorm|GoLand|CLion|DataGrip|PhpStorm|RubyMine|RustRover|Rider|AppCode|JetBrains Toolbox",
    )
    .unwrap()
});

/// 注册表 Uninstall 项扫描结果（与 PowerShell 输出的 JSON 字段一一对应）
#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RegistryUninstallItem {
    #[serde(default)]
    product_name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    install_location: String,
    #[serde(default)]
    uninstall_string: String,
    #[serde(default)]
    quiet_uninstall_string: String,
    #[serde(default)]
    publisher: String,
    #[serde(default)]
    is_toolbox: bool,
}

/// 根据 DisplayName / Publisher 推断产品代号（IIU/PCP/WS 等）
pub fn infer_product_code(display_name: &str) -> String {
    let n = display_name.to_lowercase();
    if n.contains("intellij idea") {
        if n.contains("community") {
            "IIC".into()
        } else {
            "IIU".into()
        }
    } else if n.contains("pycharm") {
        if n.contains("community") {
            "PCC".into()
        } else {
            "PCP".into()
        }
    } else if n.contains("webstorm") {
        "WS".into()
    } else if n.contains("goland") {
        "GO".into()
    } else if n.contains("clion") {
        "CL".into()
    } else if n.contains("datagrip") {
        "DG".into()
    } else if n.contains("phpstorm") {
        "PS".into()
    } else if n.contains("rubymine") {
        "RM".into()
    } else if n.contains("rustrover") {
        "RR".into()
    } else if n.contains("rider") {
        "RD".into()
    } else if n.contains("appcode") {
        "AC".into()
    } else if n.contains("toolbox") {
        "TBA".into()
    } else {
        String::new()
    }
}

/// 扫描注册表 Uninstall 项，找出已安装的 JetBrains 产品。
///
/// 返回 Result：`Ok(Vec::new())` 表示"未安装"，属正常；注册表扫描异常（非零退出码）
/// 也视为未安装并以 warn 日志上报。仅 fatal 错误（如 PowerShell 不可用）返回 Err。
pub async fn detect_jetbrains(app_handle: AppHandle) -> Result<Vec<JetBrainsInstallation>, String> {
    logger::info(&app_handle, "开始检测已安装的 JetBrains 产品...");

    let script = r#"
        $paths = @(
            'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
            'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
            'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
        )
        foreach ($path in $paths) {
            Get-ItemProperty $path -ErrorAction SilentlyContinue | Where-Object {
                $_.Publisher -eq 'JetBrains' -or ($_.DisplayName -and $_.DisplayName -match 'IntelliJ|PyCharm|WebStorm|GoLand|CLion|DataGrip|PhpStorm|RubyMine|RustRover|Rider|AppCode|JetBrains Toolbox')
            } | ForEach-Object {
                $loc = $_.InstallLocation
                [PSCustomObject]@{
                    ProductName = if ($_.DisplayName) { $_.DisplayName } else { '' }
                    Version = if ($_.DisplayVersion) { $_.DisplayVersion } else { '' }
                    InstallLocation = if ($loc) { $loc } else { '' }
                    UninstallString = if ($_.UninstallString) { $_.UninstallString } else { '' }
                    QuietUninstallString = if ($_.QuietUninstallString) { $_.QuietUninstallString } else { '' }
                    Publisher = if ($_.Publisher) { $_.Publisher } else { '' }
                    IsToolbox = if ($loc) { $loc -like '*\Toolbox\*' } else { $false }
                } | ConvertTo-Json -Compress -Depth 2
            }
        }
    "#;

    let output = process_manager::execute_powershell_env(script, &Default::default(), 60).await;
    let raw = match output {
        Ok(o) if o.exit_code == 0 => o.stdout,
        Ok(o) => {
            // 注册表扫描异常（非零退出码）：视为未安装，不中断
            logger::warn(&app_handle, &format!("注册表扫描异常: {}", o.stderr.trim()));
            return Ok(Vec::new());
        }
        Err(e) => {
            // fatal 错误（如 PowerShell 不可用）：返回 Err
            logger::warn(&app_handle, &format!("注册表扫描失败: {}", e));
            return Err(format!("注册表扫描失败: {}", e));
        }
    };

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut installs: Vec<JetBrainsInstallation> = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line == "null" {
            continue;
        }
        // PowerShell 单对象 ConvertTo-Json 在 -Compress 下是单行；多字段时也是单行
        let item: RegistryUninstallItem = match serde_json::from_str(line) {
            Ok(it) => it,
            Err(_) => continue,
        };
        if !INSTALLED_REGEX.is_match(&item.product_name) && item.publisher != "JetBrains" {
            continue;
        }
        let code = infer_product_code(&item.product_name);
        let key = format!("{}|{}|{}", item.product_name, item.version, item.install_location);
        if !seen.insert(key) {
            continue;
        }
        installs.push(JetBrainsInstallation {
            product_name: item.product_name,
            product_code: code,
            version: item.version,
            install_location: item.install_location,
            uninstall_string: item.uninstall_string,
            quiet_uninstall_string: item.quiet_uninstall_string,
            is_toolbox: item.is_toolbox,
            publisher: item.publisher,
        });
    }

    // 按 product_name 排序，便于前端展示
    installs.sort_by(|a, b| a.product_name.to_lowercase().cmp(&b.product_name.to_lowercase()));

    logger::info(
        &app_handle,
        &format!("JetBrains 检测完成，共 {} 个已安装产品", installs.len()),
    );
    Ok(installs)
}
