use crate::plugin::{ToolFeature, ToolPlugin};
use async_trait::async_trait;

pub struct PythonPlugin;

#[async_trait]
impl ToolPlugin for PythonPlugin {
    fn id(&self) -> &str { "python" }
    fn name(&self) -> &str { "Python" }
    fn icon(&self) -> &str { "code" }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "version-check".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "available-versions".into(), name: "可用版本".into(), icon: "download".into() },
            ToolFeature { id: "env-list".into(), name: "环境列表".into(), icon: "settings".into() },
            ToolFeature { id: "package-manage".into(), name: "包管理".into(), icon: "box".into() },
            ToolFeature { id: "pip-mirror".into(), name: "镜像源".into(), icon: "globe".into() },
        ]
    }

    fn command_names(&self) -> Vec<&str> {
        vec![
            "detect_python_versions", "detect_default_python",
            "list_python_environments", "list_python_packages",
            "list_pip_mirrors", "switch_pip_mirror",
            "get_available_python_versions", "download_python_only",
        ]
    }

    fn needs_admin(&self, command: &str) -> bool {
        matches!(command, "switch_pip_mirror" | "download_python_only")
    }
}
