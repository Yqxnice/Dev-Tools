use super::commands;
use crate::plugin::{ToolFeature, ToolPlugin};
use tauri::{ipc::Invoke, Wry};

pub struct PythonPlugin;

impl ToolPlugin for PythonPlugin {
    fn id(&self) -> &str { "python" }
    fn name(&self) -> &str { "Python" }
    fn icon(&self) -> &str { "code" }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "instances".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "downloads".into(), name: "可用版本".into(), icon: "download".into() },
            ToolFeature { id: "envs".into(), name: "环境列表".into(), icon: "settings".into() },
            ToolFeature { id: "packages".into(), name: "已安装包".into(), icon: "box".into() },
            ToolFeature { id: "mirror".into(), name: "镜像源".into(), icon: "globe".into() },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::detect_python,
            commands::detect_default_python,
            commands::list_python_environments,
            commands::list_python_packages,
            commands::list_python_mirrors,
            commands::switch_python_mirror,
            commands::get_available_python_versions,
            commands::download_python,
            commands::get_python_download_url,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "detect_python",
            "detect_default_python",
            "list_python_environments",
            "list_python_packages",
            "list_python_mirrors",
            "switch_python_mirror",
            "get_available_python_versions",
            "download_python",
            "get_python_download_url",
        ]
    }
}
