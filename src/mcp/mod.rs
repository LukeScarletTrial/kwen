pub mod client;
pub mod bridge;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::sync::Arc;
use anyhow::Result;

pub use client::McpClient;
pub use bridge::McpBridgeTool;
use crate::tools::ToolRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

pub struct McpManager {
    config_path: PathBuf,
    pub config: McpConfig,
}

impl McpManager {
    pub fn load_or_init() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kwen");

        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }

        let config_path = config_dir.join("mcp_servers.json");
        let config = if config_path.exists() {
            let data = fs::read_to_string(&config_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            let default_config = McpConfig::default();
            let _ = fs::write(&config_path, serde_json::to_string_pretty(&default_config)?);
            default_config
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn add_server(&mut self, name: &str, server_config: McpServerConfig) -> Result<()> {
        self.config.mcp_servers.insert(name.to_string(), server_config);
        self.save()?;
        Ok(())
    }

    pub fn remove_server(&mut self, name: &str) -> Result<bool> {
        let removed = self.config.mcp_servers.remove(name).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn save(&self) -> Result<()> {
        let json_str = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, json_str)?;
        Ok(())
    }

    pub fn list_servers(&self) -> Vec<(String, McpServerConfig)> {
        self.config.mcp_servers.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub async fn load_active_mcp_tools(&self, registry: &mut ToolRegistry) {
        for (server_name, srv_conf) in &self.config.mcp_servers {
            match McpClient::spawn(server_name, srv_conf).await {
                Ok(client) => {
                    let client_arc = Arc::new(client);
                    match client_arc.list_tools().await {
                        Ok(tools) => {
                            for t in tools {
                                if let Some(t_name) = t.get("name").and_then(|n| n.as_str()) {
                                    let t_desc = t.get("description")
                                        .and_then(|d| d.as_str())
                                        .unwrap_or("No description provided");
                                    let t_schema = t.get("inputSchema")
                                        .cloned()
                                        .unwrap_or_else(|| serde_json::json!({"type": "object"}));
                                    
                                    let prefixed_name = format!("{}_{}", server_name, t_name);
                                    let bridge = McpBridgeTool::new(
                                        Arc::clone(&client_arc),
                                        prefixed_name,
                                        t_desc.to_string(),
                                        t_schema,
                                    );
                                    registry.register(Box::new(bridge));
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to list tools for MCP server '{}': {}", server_name, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to connect to MCP server '{}': {}", server_name, e);
                }
            }
        }
    }
}

pub struct CatalogItem {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub command: &'static str,
    pub default_args: &'static [&'static str],
}

pub fn get_curated_mcp_library() -> Vec<CatalogItem> {
    vec![
        CatalogItem {
            id: "fetch",
            name: "Fetch & Web Scraping MCP",
            description: "Fetches web pages, extracts markdown, and queries web endpoints directly.",
            command: "uvx",
            default_args: &["mcp-server-fetch"],
        },
        CatalogItem {
            id: "sqlite",
            name: "SQLite Database MCP",
            description: "Inspect schema, execute read/write queries, and inspect local SQLite databases.",
            command: "uvx",
            default_args: &["mcp-server-sqlite", "--db-path", "local.db"],
        },
        CatalogItem {
            id: "git",
            name: "Git Repository MCP",
            description: "Detailed Git integration: commits, diffs, log inspection, branch operations.",
            command: "uvx",
            default_args: &["mcp-server-git"],
        },
        CatalogItem {
            id: "filesystem",
            name: "Extended Filesystem MCP",
            description: "Advanced filesystem exploration, file moves, batch inspection.",
            command: "npx",
            default_args: &["-y", "@modelcontextprotocol/server-filesystem", "."],
        },
        CatalogItem {
            id: "github",
            name: "GitHub MCP",
            description: "Manage GitHub PRs, issues, repository search, and reviews.",
            command: "npx",
            default_args: &["-y", "@modelcontextprotocol/server-github"],
        },
        CatalogItem {
            id: "memory",
            name: "Knowledge Graph Memory MCP",
            description: "Persistent memory using knowledge graph for entities and relations across sessions.",
            command: "npx",
            default_args: &["-y", "@modelcontextprotocol/server-memory"],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}
