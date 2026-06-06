use super::types::LogMessage;
use tauri::AppHandle;
use tauri::Emitter;

pub fn send_log(app_handle: &AppHandle, level: &str, message: &str) {
    match level {
        "error" => eprintln!("[ERROR] {}", message),
        "warn" => eprintln!("[WARN] {}", message),
        "info" => println!("[INFO] {}", message),
        _ => println!("[{}] {}", level, message),
    }

    let log_message = LogMessage {
        level: level.to_string(),
        message: message.to_string(),
    };
    let _ = app_handle.emit("log-message", log_message);
}

pub fn info(app_handle: &AppHandle, message: &str) {
    send_log(app_handle, "info", message);
}

pub fn error(app_handle: &AppHandle, message: &str) {
    send_log(app_handle, "error", message);
}

pub fn warn(app_handle: &AppHandle, message: &str) {
    send_log(app_handle, "warn", message);
}
