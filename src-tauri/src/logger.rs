use super::types::LogMessage;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri::Emitter;

/// 后端日志缓冲上限：避免长跑后无限增长导致 OOM。
/// 前端 loggerStore 有独立的可配置上限（50-2000），后端这里固定 1000 条兜底。
const BACKEND_LOG_MAX: usize = 1000;

static LOG_BUFFER: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// 将日志写入后端环形缓冲，并在缓冲超限时丢弃最旧的条目。
/// 缓冲本身仅用于未来可能的导出/诊断；主路径仍是 emit 到前端。
fn push_buffer(level: &str, message: &str) {
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        buf.push((level.to_string(), message.to_string()));
        if buf.len() > BACKEND_LOG_MAX {
            let drop = buf.len() - BACKEND_LOG_MAX;
            buf.drain(0..drop);
        }
    }
}

pub fn send_log(app_handle: &AppHandle, level: &str, message: &str) {
    match level {
        "error" => eprintln!("[ERROR] {}", message),
        "warn" => eprintln!("[WARN] {}", message),
        "info" => println!("[INFO] {}", message),
        _ => println!("[{}] {}", level, message),
    }

    // 后端缓冲封顶，避免长跑 OOM
    push_buffer(level, message);

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
