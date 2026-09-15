use super::commands;
use crate::plugin::{ToolFeature, ToolPlugin};
use tauri::{ipc::Invoke, Wry};

pub struct PostgresqlPlugin;

impl ToolPlugin for PostgresqlPlugin {
    fn id(&self) -> &str {
        "postgresql"
    }
    fn name(&self) -> &str {
        "PostgreSQL"
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
            commands::detect_postgresql,
            commands::get_available_postgresql_versions,
            commands::download_postgresql,
            commands::uninstall_postgresql,
            commands::scan_postgresql_residuals,
            commands::clean_postgresql_residuals,
            commands::reset_postgresql_password,
            commands::change_postgresql_password,
            commands::start_postgresql_service,
            commands::stop_postgresql_service,
        ])
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[
            "detect_postgresql",
            "get_available_postgresql_versions",
            "download_postgresql",
            "uninstall_postgresql",
            "scan_postgresql_residuals",
            "clean_postgresql_residuals",
            "reset_postgresql_password",
            "change_postgresql_password",
            "start_postgresql_service",
            "stop_postgresql_service",
        ]
    }
}
