use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;

pub mod file_tools;
pub mod bash_tools;
pub mod search_tools;

pub use file_tools::{ReadFileTool, WriteFileTool, EditFileTool, ListDirTool};
pub use bash_tools::ExecuteCommandTool;
pub use search_tools::{GrepSearchTool, FindFilesTool};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    async fn execute(&self, arguments: Value) -> Result<String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get_definitions(&self) -> Vec<crate::lmstudio::ToolDefinition> {
        self.tools
            .values()
            .map(|t| crate::lmstudio::ToolDefinition {
                tool_type: "function".to_string(),
                function: crate::lmstudio::FunctionDefinition {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    parameters: t.parameters(),
                },
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, args_json: &str) -> Result<String> {
        if let Some(tool) = self.tools.get(name) {
            let args: Value = serde_json::from_str(args_json)
                .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
            tool.execute(args).await
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }

    pub fn list_tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}
