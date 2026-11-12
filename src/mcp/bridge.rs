use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;
use std::sync::Arc;
use crate::tools::Tool;
use crate::mcp::McpClient;

pub struct McpBridgeTool {
    client: Arc<McpClient>,
    tool_name: String,
    tool_description: String,
    tool_schema: Value,
}

impl McpBridgeTool {
    pub fn new(client: Arc<McpClient>, tool_name: String, tool_description: String, tool_schema: Value) -> Self {
        Self {
            client,
            tool_name,
            tool_description,
            tool_schema,
        }
    }
}

#[async_trait]
impl Tool for McpBridgeTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn parameters(&self) -> Value {
        self.tool_schema.clone()
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        self.client.call_tool(&self.tool_name, arguments).await
    }
}
