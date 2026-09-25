#[cfg(not(target_os = "windows"))]
compile_error!("Dev Tools 仅支持 Windows 平台");

pub mod access;
pub mod types;
pub mod error;
pub mod logger;
pub mod process_manager;
pub mod service_manager;
pub mod detector_base;
pub mod http_client;
pub mod runtime_switcher;
pub mod auto_update;
pub mod env;
pub mod plugin;
pub mod mysql;
pub mod postgresql;
pub mod python;
pub mod node;
pub mod jetbrains;
pub mod java;
pub mod software;
pub mod download_control;
pub mod remote_version_cache;
pub mod system_info;

use plugin::PluginManager;
use tauri::{
    ipc::Invoke,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Wry,
};

/// 聚焦并还原主窗口（单实例二次启动 / 托盘点击时调用）
fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// ===== 全局命令（非工具插件提供） =====

#[tauri::command]
fn is_running_as_admin() -> bool {
    check_elevation::is_elevated().unwrap_or(false)
}

/// 以管理员身份重启本应用（通过 ShellExecuteW 的 "runas" 动词触发 UAC 提示）
#[tauri::command]
fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("获取当前可执行路径失败: {}", e))?
        .to_string_lossy()
        .to_string();
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;
        let exe_w = service_manager::to_wide(&exe);
        let verb_w = service_manager::to_wide("runas");
        let result = ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(verb_w.as_ptr()),
            PCWSTR(exe_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        );
        // ShellExecuteW 返回 HINSTANCE，值 <= 32 表示错误
        if result.0 as usize <= 32 {
            return Err(format!("UAC 重启失败（错误码 {}），请手动右键「以管理员身份运行」", result.0 as usize));
        }
        // 触发 UAC 后立即退出当前非提权实例
        std::process::exit(0);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = exe;
        Err("仅支持 Windows 平台".to_string())
    }
}

/// 将前端日志面板内容导出为文件，保存到「下载/DevTools/logs」，返回完整路径
#[tauri::command]
fn export_logs(content: String) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("日志为空，无可导出内容".to_string());
    }

    // 限制导出内容大小（最大 10MB），防止磁盘耗尽
    const MAX_EXPORT_BYTES: usize = 10 * 1024 * 1024;
    if trimmed.len() > MAX_EXPORT_BYTES {
        return Err(format!(
            "日志内容过大 ({} 字节，上限 {} 字节)",
            trimmed.len(),
            MAX_EXPORT_BYTES
        ));
    }

    let base_dir = download_control::devtools_log_dir();
    std::fs::create_dir_all(&base_dir)
        .map_err(|e| format!("创建日志目录失败: {}", e))?;

    let unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let file_path = base_dir.join(format!("devtools-log-{}.log", unix_secs));

    std::fs::write(&file_path, trimmed)
        .map_err(|e| format!("写入日志文件失败: {}", e))?;

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg(&base_dir)
            .spawn();
    }

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_tool_list(state: tauri::State<'_, PluginManager>) -> Vec<plugin::ToolInfo> {
    state.get_tool_list()
}

/// 获取 DevTools 下载目录（「下载/DevTools」），回退到缓存目录
#[tauri::command]
fn get_download_dir() -> String {
    download_control::devtools_download_dir().to_string_lossy().to_string()
}

/// 获取 DevTools 日志导出目录（「下载/DevTools/logs」），回退到缓存目录
#[tauri::command]
fn get_log_dir() -> String {
    download_control::devtools_log_dir().to_string_lossy().to_string()
}

/// 判断目标路径是否位于任一允许的基础目录之下（含等于基础目录本身）。
/// 双方都先 canonicalize 再做前缀匹配，消除 `..`、符号链接、8.3 短名等绕过手段；
/// `Path::starts_with` 按路径组件匹配，天然免疫 `C:\foo\bar2` 冒充 `C:\foo\bar`。
fn is_path_allowed(target: &std::path::Path, allowed_bases: &[std::path::PathBuf]) -> bool {
    let target_canonical = match target.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    allowed_bases.iter().any(|base| {
        base.canonicalize()
            .map(|b| target_canonical.starts_with(b))
            .unwrap_or(false)
    })
}

/// 在资源管理器中打开指定路径。
/// 若 path 指向文件，则打开其父目录；若 path 指向目录则直接打开。
/// 安全限制：仅允许打开 DevTools 自身目录、下载目录、日志目录下的路径，
/// 防止前端通过此命令暴露任意系统敏感目录。
#[tauri::command]
fn open_in_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 安全检查：仅允许打开 DevTools 相关目录
    let open_target = if p.is_file() {
        p.parent().map(|parent| parent.to_path_buf()).unwrap_or(p.clone())
    } else {
        p.clone()
    };

    // 获取允许的基础目录列表
    let mut allowed_bases: Vec<std::path::PathBuf> = Vec::new();

    // DevTools 下载目录
    allowed_bases.push(download_control::devtools_download_dir());
    // DevTools 日志目录
    allowed_bases.push(download_control::devtools_log_dir());
    // 应用自身目录（可执行文件所在目录）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            allowed_bases.push(parent.to_path_buf());
        }
    }

    // 检查目标路径是否在允许的目录下
    if !is_path_allowed(&open_target, &allowed_bases) {
        return Err("不允许打开此路径".to_string());
    }

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

/// 暂停下载任务
#[tauri::command]
fn pause_download(task_id: String) -> Result<(), String> {
    download_control::pause_task(&task_id)
}

/// 继续下载任务
#[tauri::command]
fn resume_download(task_id: String) -> Result<(), String> {
    download_control::resume_task(&task_id)
}

/// 取消下载任务
#[tauri::command]
fn cancel_download(task_id: String) -> Result<(), String> {
    download_control::cancel_task(&task_id)
}

/// 将前端配置 JSON 导出为文件，保存到「下载/DevTools/config」，返回完整路径
#[tauri::command]
fn export_config(content: String) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("配置内容为空，无可导出".to_string());
    }

    // 限制导出内容大小（最大 5MB），防止磁盘耗尽
    const MAX_EXPORT_BYTES: usize = 5 * 1024 * 1024;
    if trimmed.len() > MAX_EXPORT_BYTES {
        return Err(format!(
            "配置内容过大 ({} 字节，上限 {} 字节)",
            trimmed.len(),
            MAX_EXPORT_BYTES
        ));
    }

    let base_dir = download_control::devtools_config_dir();
    std::fs::create_dir_all(&base_dir)
        .map_err(|e| format!("创建配置目录失败: {}", e))?;

    let unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let file_path = base_dir.join(format!("devtools-config-{}.json", unix_secs));

    std::fs::write(&file_path, trimmed)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg(&base_dir)
            .spawn();
    }

    Ok(file_path.to_string_lossy().to_string())
}

/// 全局命令名列表（与 global_invoke_handler 中注册的命令一一对应）
const GLOBAL_COMMANDS: &[&str] = &[
    "is_running_as_admin",
    "relaunch_as_admin",
    "export_logs",
    "export_config",
    "get_tool_list",
    "get_download_dir",
    "get_log_dir",
    "open_in_folder",
    "pause_download",
    "resume_download",
    "cancel_download",
    "set_default_runtime_command",
    "check_for_updates",
    "get_system_info",
];

/// 全局命令处理器（非插件提供）
fn global_invoke_handler() -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
    Box::new(tauri::generate_handler![
        is_running_as_admin,
        relaunch_as_admin,
        export_logs,
        export_config,
        get_tool_list,
        get_download_dir,
        get_log_dir,
        open_in_folder,
        pause_download,
        resume_download,
        cancel_download,
        runtime_switcher::set_default_runtime_command,
        auto_update::check_for_updates,
        system_info::get_system_info,
    ])
}

/// 手动克隆 Invoke（Tauri 未为其 derive Clone，但所有字段均为 pub + Clone）
fn clone_invoke(invoke: &Invoke<Wry>) -> Invoke<Wry> {
    Invoke {
        message: invoke.message.clone(),
        resolver: invoke.resolver.clone(),
        acl: invoke.acl.clone(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut plugin_mgr = PluginManager::new();
    plugin_mgr.register(Box::new(mysql::MySqlPlugin));
    plugin_mgr.register(Box::new(postgresql::PostgresqlPlugin));
    plugin_mgr.register(Box::new(python::PythonPlugin));
    plugin_mgr.register(Box::new(node::NodePlugin));
    plugin_mgr.register(Box::new(jetbrains::JetBrainsPlugin));
    plugin_mgr.register(Box::new(java::JavaPlugin));
    plugin_mgr.register(Box::new(env::EnvPlugin));
    plugin_mgr.register(Box::new(software::SoftwarePlugin));

    // 构建命令路由表：命令名 -> handler 索引（O(1) 查找）
    let (mut route, mut handlers) = plugin_mgr.build_router();
    // 全局命令注册为最后一个 handler（作为已知全局命令的快速入口）
    let global_idx = handlers.len();
    handlers.push(global_invoke_handler());
    for name in GLOBAL_COMMANDS {
        route.insert(*name, global_idx);
    }

    // 命令名命中路由表则直接调用对应 handler，无需线性遍历所有插件
    let invoke_handler = move |invoke: Invoke<Wry>| {
        let cmd = invoke.message.command();
        if let Some(&idx) = route.get(cmd) {
            return handlers[idx](clone_invoke(&invoke));
        }
        false
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            focus_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(plugin_mgr)
        .invoke_handler(invoke_handler)
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let mut builder = TrayIconBuilder::with_id("main-tray")
                .tooltip("Dev Tools - 本地开发环境管理")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => focus_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        focus_main_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                builder = builder.icon(icon.clone());
            }
            builder.build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            // 启动失败时不静默 panic：写日志到 stderr 并弹出 Windows 原生消息框
            eprintln!("[FATAL] Tauri 启动失败: {}", e);
            let msg = format!(
                "Dev Tools 启动失败：{}\n\n常见原因：\n• 已有实例运行\n• WebView2 运行时缺失\n• 托盘创建失败\n\n请确认环境后重试。",
                e
            );
            // 用 Windows 原生 MessageBox 兜底（不依赖 Tauri 已挂掉的运行时）
            #[cfg(target_os = "windows")]
            unsafe {
                use windows::core::PCSTR;
                use windows::Win32::Foundation::HWND;
                use windows::Win32::UI::WindowsAndMessaging::{MessageBoxA, MB_ICONERROR, MB_OK};
                let title = b"Dev Tools \0";
                let body = format!("{}\0", msg);
                MessageBoxA(
                    HWND(std::ptr::null_mut()),
                    PCSTR(body.as_ptr()),
                    PCSTR(title.as_ptr()),
                    MB_ICONERROR | MB_OK,
                );
            }
            // MessageBoxA 返回后仍要退出（非 0 退出码便于脚本捕获）
            std::process::exit(1);
        });
}

#[cfg(test)]
mod tests {
    use super::is_path_allowed;
    use std::path::PathBuf;

    /// 在系统临时目录下创建独立的测试目录树，返回 (base, base/sub, allowed_evil 同前缀干扰目录)。
    /// name 用于隔离并行执行的各测试用例，避免互相删除目录。
    fn setup_dirs(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("devtools_test_{}_{}", std::process::id(), name));
        let base = root.join("allowed");
        let sub = base.join("nested").join("deep");
        let impostor = root.join("allowed_evil"); // 与 base 共享字符串前缀但非子目录
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir_all(&impostor).unwrap();
        (base, sub, impostor)
    }

    fn teardown(base: &std::path::Path) {
        if let Some(root) = base.parent().and_then(|p| p.parent()) {
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    fn allows_base_dir_itself() {
        let (base, _sub, _impostor) = setup_dirs("base_itself");
        assert!(is_path_allowed(&base, &[base.clone()]));
        teardown(&base);
    }

    #[test]
    fn allows_nested_subdirectory() {
        let (base, sub, _impostor) = setup_dirs("nested_sub");
        assert!(is_path_allowed(&sub, &[base.clone()]));
        teardown(&base);
    }

    #[test]
    fn rejects_string_prefix_impostor_dir() {
        // allowed_evil 与 allowed 共享字符串前缀，但按路径组件匹配必须拒绝
        let (base, _sub, impostor) = setup_dirs("impostor");
        assert!(!is_path_allowed(&impostor, &[base.clone()]));
        teardown(&base);
    }

    #[test]
    fn rejects_dotdot_traversal() {
        // base/../allowed_evil canonicalize 后落在白名单外
        let (base, _sub, impostor) = setup_dirs("dotdot");
        let traversal = base.join("..").join("allowed_evil");
        assert!(!is_path_allowed(&traversal, &[base.clone()]));
        // 自我校验：traversal 解析后确实等于 impostor
        assert_eq!(traversal.canonicalize().unwrap(), impostor.canonicalize().unwrap());
        teardown(&base);
    }

    #[test]
    fn rejects_unrelated_system_dir() {
        let (base, _sub, _impostor) = setup_dirs("unrelated");
        let system_dir = std::env::temp_dir().join("..").canonicalize().unwrap();
        assert!(!is_path_allowed(&system_dir, &[base.clone()]));
        teardown(&base);
    }

    #[test]
    fn rejects_nonexistent_target() {
        let (base, _sub, _impostor) = setup_dirs("ghost_target");
        let ghost = base.join("does_not_exist_at_all");
        assert!(!is_path_allowed(&ghost, &[base.clone()]));
        teardown(&base);
    }

    #[test]
    fn rejects_when_base_cannot_be_canonicalized() {
        let (base, sub, _impostor) = setup_dirs("ghost_base");
        let ghost_base = base.parent().unwrap().join("nonexistent_base_dir");
        assert!(!is_path_allowed(&sub, &[ghost_base]));
        teardown(&base);
    }
}
