use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{ipc::Invoke, Wry};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct ToolFeature {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../src/types/generated/")]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub features: Vec<ToolFeature>,
}

/// 工具插件：提供菜单元数据 + IPC 命令处理器
///
/// 命令通过 invoke_handler() 返回一个闭包，由 lib.rs::run() 统一分发。
/// 新增工具只需实现本 trait，无需修改 lib.rs 中的命令注册逻辑。
pub trait ToolPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn features(&self) -> Vec<ToolFeature>;
    /// 返回该插件的 IPC 命令处理器（由 generate_handler! 生成的闭包）
    fn invoke_handler(&self) -> Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static>;
    /// 返回该插件处理的 IPC 命令名列表，用于构建 O(1) 路由表
    fn command_names(&self) -> &'static [&'static str];
}

#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn ToolPlugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn ToolPlugin>) {
        self.plugins.push(plugin);
    }

    pub fn get_tool_list(&self) -> Vec<ToolInfo> {
        self.plugins
            .iter()
            .map(|p| ToolInfo {
                id: p.id().to_string(),
                name: p.name().to_string(),
                icon: p.icon().to_string(),
                features: p.features(),
            })
            .collect()
    }

    /// 构建命令路由表：命令名 -> 对应 handler 在返回 Vec 中的索引。
    /// 调用方拿到 (route, handlers) 后，IPC 调用先 `route.get(command)` 命中索引，
    /// 再 `handlers[idx](invoke)`，避免线性遍历所有插件的 handler 闭包。
    pub fn build_router(
        &self,
    ) -> (
        HashMap<&'static str, usize>,
        Vec<Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static>>,
    ) {
        let mut route: HashMap<&'static str, usize> = HashMap::new();
        let mut handlers: Vec<Box<dyn Fn(Invoke<Wry>) -> bool + Send + Sync + 'static>> =
            Vec::new();
        for p in &self.plugins {
            let idx = handlers.len();
            handlers.push(p.invoke_handler());
            for name in p.command_names() {
                route.insert(*name, idx);
            }
        }
        (route, handlers)
    }
}
