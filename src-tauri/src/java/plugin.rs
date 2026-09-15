use super::commands;
use crate::plugin::{ToolFeature, ToolPlugin};
use tauri::{ipc::Invoke, Wry};

pub struct JavaPlugin;

impl ToolPlugin for JavaPlugin {
    fn id(&self) -> &str {
        "java"
    }
    fn name(&self) -> &str {
        "Java"
    }
    fn icon(&self) -> &str {
        "code"
    }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "instances".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "downloads".into(), name: "可用版本".into(), icon: "download".into() },
            ToolFeature { id: "mirror".into(), name: "Maven 镜像".into(), icon: "globe".into() },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::detect_java,
            commands::detect_default_java,
            commands::get_available_java_versions,
            commands::get_java_download_url,
            commands::list_java_mirrors,
            commands::switch_java_mirror,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "detect_java",
            "detect_default_java",
            "get_available_java_versions",
            "get_java_download_url",
            "list_java_mirrors",
            "switch_java_mirror",
        ]
    }
}
