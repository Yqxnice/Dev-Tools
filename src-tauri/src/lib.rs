#[cfg(not(target_os = "windows"))]
compile_error!("Dev Tools 仅支持 Windows 平台");

pub mod access;
pub mod types;
pub mod error;
pub mod logger;
pub mod process_manager;
pub mod service_manager;
pub mod detector_base;
pub mod plugin;
pub mod mysql;
pub mod postgresql;
pub mod python;
pub mod node;
pub mod jetbrains;
pub mod java;
pub mod download_control;

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
        let exe_w = to_wide(&exe);
        let verb_w = to_wide("runas");
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

/// 将 &str 转为以 null 结尾的 UTF-16 宽字符序列
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 将前端日志面板内容导出为文件，保存到「下载/DevTools/logs」，返回完整路径
#[tauri::command]
fn export_logs(content: String) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("日志为空，无可导出内容".to_string());
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

/// 在资源管理器中打开指定路径。
/// 若 path 指向文件，则打开其父目录；若 path 指向目录则直接打开。
#[tauri::command]
fn open_in_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    let open_target = if p.is_file() {
        p.parent().map(|parent| parent.to_path_buf()).unwrap_or(p)
    } else {
        p
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

/// 全局命令名列表（与 global_invoke_handler 中注册的命令一一对应）
const GLOBAL_COMMANDS: &[&str] = &[
    "is_running_as_admin",
    "relaunch_as_admin",
    "export_logs",
    "get_tool_list",
    "get_download_dir",
    "get_log_dir",
    "open_in_folder",
    "pause_download",
    "resume_download",
    "cancel_download",
];

/// 全局命令处理器（非插件提供）
fn global_invoke_handler() -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
    Box::new(tauri::generate_handler![
        is_running_as_admin,
        relaunch_as_admin,
        export_logs,
        get_tool_list,
        get_download_dir,
        get_log_dir,
        open_in_folder,
        pause_download,
        resume_download,
        cancel_download,
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
