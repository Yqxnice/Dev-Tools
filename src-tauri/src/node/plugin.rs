use super::commands;
use crate::plugin::{ToolFeature, ToolPlugin};
use tauri::{ipc::Invoke, Wry};

pub struct NodePlugin;

impl ToolPlugin for NodePlugin {
    fn id(&self) -> &str {
        "node"
    }
    fn name(&self) -> &str {
        "Node.js"
    }
    fn icon(&self) -> &str {
        "code"
    }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "instances".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "downloads".into(), name: "可用版本".into(), icon: "download".into() },
            ToolFeature { id: "envs".into(), name: "环境列表".into(), icon: "settings".into() },
            ToolFeature { id: "packages".into(), name: "全局包".into(), icon: "box".into() },
            ToolFeature { id: "mirror".into(), name: "镜像源".into(), icon: "globe".into() },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::detect_node,
            commands::detect_default_node,
            commands::list_node_packages,
            commands::list_node_mirrors,
            commands::switch_node_mirror,
            commands::get_available_node_versions,
            commands::get_node_download_url,
            commands::download_node,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "detect_node",
            "detect_default_node",
            "list_node_packages",
            "list_node_mirrors",
            "switch_node_mirror",
            "get_available_node_versions",
            "get_node_download_url",
            "download_node",
        ]
    }
}
