//! 危险运维操作的互斥保护（P1-5）。
//!
//! 前端可以在同一时刻连发多个 IPC 调用（双击、切换标签后误点、网络重试等），
//! 而卸载 / 残留清理 / 密码重置 / 服务启停这类命令会改写系统状态：
//! 两个 `reset_mysql_password` 并发会互相覆盖 `my.ini`、互相杀进程，
//! 最终把实例留在半配置状态；`clean_*` 与 `uninstall_*` 并发则可能
//! 对同一目录做删除竞赛（一个删到一半另一个又在删）。
//!
//! 设计取舍：**try_lock 快速失败，而不是排队等待**。
//! - 排队会让用户以为第二个操作"成功了"，实际它在几十秒后才开始，
//!   期间第一个操作可能已经改了前置条件（服务已停、目录已删）。
//! - 快速失败给出确定的错误信息，前端可直接提示"上一个操作尚未完成"。
//!
//! 按工具划分作用域：MySQL 与 PostgreSQL 的操作互不阻塞。
//! Guard 只持有 `Scope`（一个 `Copy` 枚举），因此可以安全地跨 `.await` 持有，
//! 且实现里没有跨 await 持有 `std::sync::MutexGuard`（不会违反 Send 约束）。

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// 互斥作用域：同一作用域内同一时刻只允许一个操作
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Scope {
    Mysql,
    Postgresql,
    JetBrains,
}

impl Scope {
    /// 用于错误提示的可读名称
    pub fn label(self) -> &'static str {
        match self {
            Scope::Mysql => "MySQL",
            Scope::Postgresql => "PostgreSQL",
            Scope::JetBrains => "JetBrains",
        }
    }
}

fn active() -> MutexGuard<'static, HashSet<Scope>> {
    static ACTIVE: OnceLock<Mutex<HashSet<Scope>>> = OnceLock::new();
    ACTIVE
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 运维操作守卫：drop（含正常返回、`?` 提前退出、panic unwind）时自动释放作用域。
///
/// **不要**把它存进结构体长期持有，也不要在持有时再调用同一个作用域的
/// 另一个受保护命令 —— 会立即失败（快速失败而非死锁，所以不会卡死）。
#[derive(Debug)]
pub struct MaintenanceGuard {
    scope: Scope,
}

impl MaintenanceGuard {
    pub fn scope(&self) -> Scope {
        self.scope
    }
}

impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        let mut set = active();
        set.remove(&self.scope);
    }
}

/// 尝试占用作用域；已被占用时立即返回错误。
pub fn try_acquire(scope: Scope) -> Result<MaintenanceGuard, String> {
    let mut set = active();
    if set.contains(&scope) {
        return Err(format!(
            "已有 {} 维护操作执行中，请等待其完成后再试",
            scope.label()
        ));
    }
    set.insert(scope);
    Ok(MaintenanceGuard { scope })
}

/// 当前是否有操作占用该作用域（测试/诊断用）
pub fn is_busy(scope: Scope) -> bool {
    active().contains(&scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// 全局作用域表是共享的，而 `cargo test` 默认并行跑测试 ——
    /// 不同测试用到同一个 Scope 时会互相抢锁导致偶发失败。
    /// 这里用一把测试锁把本模块内的测试串行化。
    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    fn acquire_then_release_by_drop() {
        let _s = serial();
        {
            let guard = try_acquire(Scope::Mysql).expect("首次获取应成功");
            assert!(is_busy(Scope::Mysql));
            assert_eq!(guard.scope(), Scope::Mysql);
            // 另一个作用域不受影响
            assert!(!is_busy(Scope::Postgresql));
        }
        assert!(!is_busy(Scope::Mysql), "drop 后必须释放");
    }

    #[test]
    fn second_acquire_in_same_scope_fails_fast() {
        let _s = serial();
        let first = try_acquire(Scope::JetBrains).expect("首次获取应成功");
        let err = try_acquire(Scope::JetBrains).expect_err("并发获取应失败");
        assert!(err.contains("JetBrains"), "错误应指明工具: {}", err);
        assert!(err.contains("执行中"));
        drop(first);
        assert!(try_acquire(Scope::JetBrains).is_ok(), "释放后应可再次获取");
    }

    #[test]
    fn scopes_are_independent() {
        let _s = serial();
        let _mysql = try_acquire(Scope::Mysql).expect("获取 MySQL 应成功");
        let _pg = try_acquire(Scope::Postgresql).expect("获取 PostgreSQL 应成功");
        let _jb = try_acquire(Scope::JetBrains).expect("获取 JetBrains 应成功");
        assert!(try_acquire(Scope::Mysql).is_err());
    }

    #[test]
    fn early_return_via_question_mark_releases() {
        let _s = serial();
        fn inner() -> Result<(), String> {
            let _guard = try_acquire(Scope::Postgresql)?;
            Err("提前退出".to_string())
        }
        assert!(inner().is_err());
        assert!(!is_busy(Scope::Postgresql), "`?` 提前返回必须释放");
    }

    #[test]
    fn works_across_threads() {
        let _s = serial();
        // 主线程持有期间，另一线程必须拿不到（不靠 sleep 竞态）
        let holder = try_acquire(Scope::Mysql).expect("主线程先获取");
        let other = thread::spawn(|| try_acquire(Scope::Mysql).is_err());
        assert!(other.join().expect("线程未 panic"), "另一线程应获取失败");
        drop(holder);
        let after = thread::spawn(|| try_acquire(Scope::Mysql).is_ok());
        assert!(after.join().expect("线程未 panic"), "释放后另一线程应能获取");
    }

    #[test]
    fn labels_are_distinct() {
        assert_ne!(Scope::Mysql.label(), Scope::Postgresql.label());
        assert_ne!(Scope::Mysql.label(), Scope::JetBrains.label());
    }
}
