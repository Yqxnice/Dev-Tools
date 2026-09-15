use super::commands;
use crate::plugin::{ToolFeature, ToolPlugin};
use tauri::{ipc::Invoke, Wry};

pub struct MySqlPlugin;

impl ToolPlugin for MySqlPlugin {
    fn id(&self) -> &str {
        "mysql"
    }
    fn name(&self) -> &str {
        "MySQL"
    }
    fn icon(&self) -> &str {
        "database"
    }
    fn features(&self) -> Vec<ToolFeature> {
        vec![
            ToolFeature { id: "instances".into(), name: "版本检测".into(), icon: "check-circle".into() },
            ToolFeature { id: "downloads".into(), name: "可用版本".into(), icon: "download".into() },
            ToolFeature { id: "cleanup".into(), name: "卸载清理".into(), icon: "trash".into() },
            ToolFeature { id: "password".into(), name: "密码管理".into(), icon: "key".into() },
        ]
    }

    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static> {
        Box::new(tauri::generate_handler![
            commands::detect_mysql,
            commands::uninstall_mysql,
            commands::scan_mysql_residuals,
            commands::clean_mysql_residuals,
            commands::reset_mysql_password,
            commands::change_mysql_password,
            commands::start_mysql_service,
            commands::stop_mysql_service,
            commands::get_available_mysql_versions,
            commands::download_mysql,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "detect_mysql",
            "uninstall_mysql",
            "scan_mysql_residuals",
            "clean_mysql_residuals",
            "reset_mysql_password",
            "change_mysql_password",
            "start_mysql_service",
            "stop_mysql_service",
            "get_available_mysql_versions",
            "download_mysql",
        ]
    }
}
