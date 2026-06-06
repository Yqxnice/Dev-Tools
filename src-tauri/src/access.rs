use check_elevation::is_elevated;
use std::sync::atomic::{AtomicBool, Ordering};

static GUEST_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_guest_mode(enabled: bool) {
    GUEST_MODE.store(enabled, Ordering::SeqCst);
}

pub fn is_guest_mode() -> bool {
    GUEST_MODE.load(Ordering::SeqCst)
}

pub fn require_write_access() -> Result<(), String> {
    if is_guest_mode() {
        return Err("游客模式仅支持检测和查看，请退出游客模式后重试".to_string());
    }
    Ok(())
}

pub fn require_admin() -> Result<(), String> {
    require_write_access()?;
    if is_elevated().unwrap_or(false) {
        Ok(())
    } else {
        Err("此操作需要管理员权限，请以管理员身份运行应用".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_mode_blocks_write_access() {
        set_guest_mode(true);
        assert!(require_write_access().is_err());
        set_guest_mode(false);
        assert!(require_write_access().is_ok());
    }
}
