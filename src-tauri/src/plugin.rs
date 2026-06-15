use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFeature {
    pub id: String,
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub features: Vec<ToolFeature>,
}

#[async_trait]
pub trait ToolPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn features(&self) -> Vec<ToolFeature>;

    fn command_names(&self) -> Vec<&str>;
    fn needs_admin(&self, command: &str) -> bool;
}

pub struct PluginManager {
    plugins: Vec<Box<dyn ToolPlugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
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

    pub fn get_plugin(&self, id: &str) -> Option<&Box<dyn ToolPlugin>> {
        self.plugins.iter().find(|p| p.id() == id)
    }

    pub fn all_command_names(&self) -> Vec<&str> {
        self.plugins.iter().flat_map(|p| p.command_names()).collect()
    }

    pub fn all_features(&self) -> Vec<ToolFeature> {
        self.plugins
            .iter()
            .flat_map(|p| p.features())
            .collect()
    }
}
