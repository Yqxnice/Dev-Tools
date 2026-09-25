//! 环境变量管理器（Feature 3）
//!
//! 设计目标：可视化查看系统 PATH 与常见环境变量，检测冲突。
//! MVP 范围：只读 + 冲突检测；不修改注册表，不需要管理员权限。
//!
//! 模块结构：
//! - `detector.rs`：读取注册表 HKLM + HKCU 的 PATH 与常见 env 变量
//! - `commands.rs`：IPC 命令封装
//! - `plugin.rs`：ToolPlugin trait 实现

mod detector;
mod commands;
mod plugin;

pub use plugin::EnvPlugin;
