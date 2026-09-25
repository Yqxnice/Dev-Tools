use crate::plugin::{ToolFeature, ToolPlugin};
use super::commands;
use tauri::{ipc::Invoke, Wry};

/// 软件：软件导航 + 教程。仅一个图标兜底解析命令，无其他后端逻辑
pub struct SoftwarePlugin;

impl ToolPlugin for SoftwarePlugin {
    fn id(&self) -> &str {
        "software"
    }
    fn name(&self) -> &str {
        "软件"
    }
    fn icon(&self) -> &str {
        "apps"
    }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "list".into(), name: "软件列表".into(), icon: "download".into() },
            ToolFeature { id: "tutorials".into(), name: "教程".into(), icon: "globe".into() },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::resolve_software_icon,
            commands::check_software_installed,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &["resolve_software_icon", "check_software_installed"]
    }
}
