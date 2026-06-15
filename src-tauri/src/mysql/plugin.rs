use crate::plugin::{ToolFeature, ToolPlugin};
use async_trait::async_trait;

pub struct MySqlPlugin;

#[async_trait]
impl ToolPlugin for MySqlPlugin {
    fn id(&self) -> &str { "mysql" }
    fn name(&self) -> &str { "MySQL" }
    fn icon(&self) -> &str { "database" }

    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "version-check".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "auto-uninstall".into(), name: "自动卸载".into(), icon: "trash".into() },
            ToolFeature { id: "residue-clear".into(), name: "残留清除".into(), icon: "broom".into() },
            ToolFeature { id: "password-reset".into(), name: "密码重置".into(), icon: "key".into() },
            ToolFeature { id: "password-change".into(), name: "密码修改".into(), icon: "edit-key".into() },
        ]
    }

    fn command_names(&self) -> Vec<&str> {
        vec![
            "detect_mysql", "uninstall_mysql", "scan_mysql_residuals",
            "clean_mysql_residuals", "reset_mysql_password",
            "change_mysql_password", "start_mysql_service", "stop_mysql_service",
        ]
    }

    fn needs_admin(&self, command: &str) -> bool {
        !matches!(command, "detect_mysql")
    }
}
