use super::commands;
use crate::plugin::{ToolFeature, ToolPlugin};
use tauri::{ipc::Invoke, Wry};

pub struct JetBrainsPlugin;

impl ToolPlugin for JetBrainsPlugin {
    fn id(&self) -> &str { "jetbrains" }
    fn name(&self) -> &str { "JetBrains" }
    fn icon(&self) -> &str { "rocket" }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "instances".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "downloads".into(), name: "可用版本".into(), icon: "download".into() },
            ToolFeature { id: "cleanup".into(), name: "卸载清理".into(), icon: "trash".into() },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::detect_jetbrains,
            commands::get_available_jetbrains_versions,
            commands::get_jetbrains_product_versions,
            commands::download_jetbrains,
            commands::uninstall_jetbrains,
            commands::scan_jetbrains_residuals,
            commands::clean_jetbrains_residuals,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "detect_jetbrains",
            "get_available_jetbrains_versions",
            "get_jetbrains_product_versions",
            "download_jetbrains",
            "uninstall_jetbrains",
            "scan_jetbrains_residuals",
            "clean_jetbrains_residuals",
        ]
    }
}
