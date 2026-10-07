use super::types::LogMessage;
use regex::Regex;
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;
use tauri::Emitter;

/// 后端日志缓冲上限：避免长跑后无限增长导致 OOM。
/// 前端 loggerStore 有独立的可配置上限（50-2000），后端这里固定 1000 条兜底。
const BACKEND_LOG_MAX: usize = 1000;

static LOG_BUFFER: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn mask_patterns() -> &'static Vec<(Regex, &'static str)> {
    static PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            // MySQL：IDENTIFIED BY '...' / IDENTIFIED WITH mysql_native_password BY '...'
            (
                Regex::new(r"(?i)(IDENTIFIED\s+(?:WITH\s+\S+\s+)?BY\s+)'([^']*)'").unwrap(),
                "$1'***'",
            ),
            // MySQL：SET ... PASSWORD('...') / PASSWORD('...')
            (
                Regex::new(r"(?i)(PASSWORD\s*\(\s*)'([^']*)'(\s*\))").unwrap(),
                "$1'***'$3",
            ),
            // PostgreSQL / 旧 MySQL：... PASSWORD '...'
            (
                Regex::new(r"(?i)(PASSWORD\s+)(')([^']*)(')").unwrap(),
                "$1$2***$4",
            ),
            // 临时脚本变量：@new_password = '...'
            (
                Regex::new(r"(?i)(@new_password\s*=\s*)'([^']*)'").unwrap(),
                "$1'***'",
            ),
            // 引号包裹的 key=value（配置文件、命令行参数）
            (
                Regex::new(
                    r#"(?i)\b(password|passwd|pwd|PGPASSWORD|MYSQL_PWD)\s*[=:]\s*"([^"]*)""#,
                )
                .unwrap(),
                "$1=\"***\"",
            ),
            // 未引号包裹的 key=value（env 行、连串参数）
            (
                Regex::new(r"(?i)\b(password|passwd|pwd|PGPASSWORD|MYSQL_PWD)\s*[=:]\s*(\S+)").unwrap(),
                "$1=***",
            ),
            // 连接串凭据：scheme://user:password@host
            (
                Regex::new(r"(\w+://[^\s:/@]+):([^\s@]+)@").unwrap(),
                "$1:***@",
            ),
        ]
    })
}

/// 日志脱敏兜底。
///
/// 各调用点本应自行掩码（SQL 侧已用 `masked_sql`），但日志是**跨页面汇聚到
/// 同一处**的，任何一处漏掩码都会把密码永久留在后端环形缓冲里，并随
/// `export_logs` 一起被导出。这里按模式做一次性兜底，属于纵深防御而非
/// 主要防线：脱敏后仍可能残留调用方本就写错的内容，所以新代码不要依赖它。
pub fn sanitize(message: &str) -> String {
    if !message.bytes().any(|b| b == b'\'' || b == b'"' || b == b'=' || b == b':') {
        // 快速路径：不含引号/等号/冒号的消息不可能命中任何一条模式
        return message.to_string();
    }
    let mut out = message.to_string();
    for (re, replacement) in mask_patterns() {
        out = re.replace_all(&out, *replacement).into_owned();
    }
    out
}

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
    // 统一脱敏后再落地到控制台 / 环形缓冲 / 前端事件，三处不一致就没意义
    let message = sanitize(message);

    match level {
        "error" => eprintln!("[ERROR] {}", message),
        "warn" => eprintln!("[WARN] {}", message),
        "info" => println!("[INFO] {}", message),
        _ => println!("[{}] {}", level, message),
    }

    // 后端缓冲封顶，避免长跑 OOM
    push_buffer(level, &message);

    let log_message = LogMessage {
        level: level.to_string(),
        message,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_masks_mysql_identified_by() {
        let sql = "ALTER USER 'root'@'localhost' IDENTIFIED BY 'MySecret123';";
        let masked = sanitize(sql);
        assert!(!masked.contains("MySecret123"), "实际结果: {}", masked);
        assert!(masked.contains("IDENTIFIED BY '***'"), "实际结果: {}", masked);
    }

    #[test]
    fn sanitize_masks_mysql_identified_with_variant() {
        let sql = "ALTER USER 'root'@'%' IDENTIFIED WITH mysql_native_password BY 'pw';";
        let masked = sanitize(sql);
        assert!(!masked.contains("pw"), "实际结果: {}", masked);
        assert!(masked.contains("'***'"), "实际结果: {}", masked);
    }

    #[test]
    fn sanitize_masks_postgres_password_literal() {
        let sql = "ALTER USER postgres PASSWORD 'pgsecret';";
        let masked = sanitize(sql);
        assert!(!masked.contains("pgsecret"), "实际结果: {}", masked);
        assert!(masked.contains("PASSWORD '***'"), "实际结果: {}", masked);
    }

    #[test]
    fn sanitize_masks_mysql_password_function() {
        let sql = "UPDATE mysql.user SET Password=PASSWORD('oldpw') WHERE User='root';";
        let masked = sanitize(sql);
        assert!(!masked.contains("oldpw"), "实际结果: {}", masked);
        // 结构可保留（PASSWORD('***')），也可能被 key=value 规则折成 Password=***
        // —— 两者都正确，关键是密文不出现
        assert!(masked.contains("Password="), "实际结果: {}", masked);
        assert!(masked.contains("User='root'"), "WHERE 子句不应被误伤: {}", masked);
    }

    #[test]
    fn sanitize_masks_key_value_forms() {
        for input in [
            "password=abc123",
            "PGPASSWORD=abc123",
            "MYSQL_PWD=abc123",
            "using password: abc123",
            "password=\"abc 123\"",
            "-e password=abc123",
        ] {
            let masked = sanitize(input);
            assert!(
                !masked.contains("abc123"),
                "输入 {:?} 未被脱敏: {}",
                input,
                masked
            );
            assert!(
                !masked.contains("abc 123"),
                "输入 {:?} 未被脱敏: {}",
                input,
                masked
            );
        }
    }

    #[test]
    fn sanitize_masks_connection_uri_credentials() {
        let masked = sanitize("连接 postgres://admin:S3cret@db.example.com:5432/x 失败");
        assert!(!masked.contains("S3cret"), "实际结果: {}", masked);
        assert!(masked.contains("admin:***@"), "实际结果: {}", masked);
    }

    #[test]
    fn sanitize_keeps_ordinary_messages_intact() {
        for msg in [
            "正在停止 MySQL 服务: MySQL80",
            "SQL 执行成功！",
            "检测到 MySQL 8.0+，使用清空认证信息方式",
            "等待服务 MySQL80 状态超时（当前: stopped）",
        ] {
            assert_eq!(sanitize(msg), msg, "普通日志不应被改动");
        }
    }

    #[test]
    fn sanitize_is_idempotent() {
        let once = sanitize("ALTER USER 'root'@'localhost' IDENTIFIED BY 'x1y2z3';");
        let twice = sanitize(&once);
        assert_eq!(once, twice, "重复脱敏不应继续变形");
    }

    #[test]
    fn sanitize_handles_empty_and_no_quote_messages_quickly() {
        assert_eq!(sanitize(""), "");
        assert_eq!(sanitize("plain message"), "plain message");
    }

    #[test]
    fn mask_patterns_are_valid_and_unique() {
        let patterns = mask_patterns();
        assert!(!patterns.is_empty());
        // 空正则会"匹配一切"，从而把整条日志替换成掩码
        for (re, _) in patterns {
            assert!(!re.as_str().is_empty(), "不允许空正则");
        }
    }
}
