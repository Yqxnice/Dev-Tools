use check_elevation::is_elevated;

/// 管理员权限硬边界：非提权进程无法真正完成系统级修改。
/// 前端 usePermission() 的判断只是 UI 引导，真正兜底在这里。
pub fn require_admin() -> Result<(), String> {
    if is_elevated().unwrap_or(false) {
        Ok(())
    } else {
        Err("此操作需要管理员权限，请以管理员身份运行应用".to_string())
    }
}
