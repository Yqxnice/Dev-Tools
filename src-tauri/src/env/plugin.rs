use crate::plugin::{ToolFeature, ToolPlugin};
use super::commands;
use tauri::{ipc::Invoke, Wry};

/// 环境变量管理器插件（Feature 3）
pub struct EnvPlugin;

impl ToolPlugin for EnvPlugin {
    fn id(&self) -> &str {
        "env"
    }
    fn name(&self) -> &str {
        "环境变量"
    }
    fn icon(&self) -> &str {
        "terminal"
    }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature {
                id: "paths".into(),
                name: "PATH 管理".into(),
                icon: "check-circle".into(),
            },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::get_env_paths,
            commands::get_env_variables,
            commands::detect_path_conflicts,
            commands::open_path_entry,
            commands::open_env_editor,
            commands::backup_env_variables,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "get_env_paths",
            "get_env_variables",
            "detect_path_conflicts",
            "open_path_entry",
            "open_env_editor",
            "backup_env_variables",
        ]
    }
}
