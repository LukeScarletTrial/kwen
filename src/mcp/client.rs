use std::process::Stdio;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{json, Value};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse, McpServerConfig};

pub struct McpClient {
    pub server_name: String,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout_reader: Arc<Mutex<BufReader<ChildStdout>>>,
    _child: Child,
    next_id: Arc<Mutex<u64>>,
}

impl McpClient {
    pub async fn spawn(name: &str, config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        for (k, v) in &config.env {
            cmd.env(k, v);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to spawn MCP server '{}' ({}): {}", name, config.command, e))?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open child stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open child stdout"))?;

        let client = Self {
            server_name: name.to_string(),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout_reader: Arc::new(Mutex::new(BufReader::new(stdout))),
            _child: child,
            next_id: Arc::new(Mutex::new(1)),
        };

        // Initialize MCP handshake
        client.initialize().await?;

        Ok(client)
    }

    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = {
            let mut id_lock = self.next_id.lock().await;
            let current = *id_lock;
            *id_lock += 1;
            current
        };

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let mut req_str = serde_json::to_string(&req)?;
        req_str.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(req_str.as_bytes()).await?;
            stdin.flush().await?;
        }

        let mut line = String::new();
        {
            let mut reader = self.stdout_reader.lock().await;
            reader.read_line(&mut line).await?;
        }

        if line.is_empty() {
            return Err(anyhow!("MCP server closed connection without responding"));
        }

        let resp: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| anyhow!("Failed to parse MCP response: {}. Raw: {}", e, line))?;

        if let Some(err) = resp.error {
            return Err(anyhow!("MCP error: {:?}", err));
        }

        Ok(resp.result.unwrap_or(Value::Null))
    }

    async fn initialize(&self) -> Result<()> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": { "listChanged": true }
            },
            "clientInfo": {
                "name": "kwen-agent",
                "version": "0.1.0"
            }
        });

        let _ = self.send_request("initialize", Some(params)).await?;

        // Send initialized notification
        let notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let mut notif_str = serde_json::to_string(&notif)?;
        notif_str.push('\n');
        let mut stdin = self.stdin.lock().await;
        let _ = stdin.write_all(notif_str.as_bytes()).await;
        let _ = stdin.flush().await;

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<Value>> {
        let res = self.send_request("tools/list", None).await?;
        if let Some(tools) = res.get("tools").and_then(|t| t.as_array()) {
            Ok(tools.clone())
        } else {
            Ok(vec![])
        }
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let res = self.send_request("tools/call", Some(params)).await?;
        if let Some(content) = res.get("content").and_then(|c| c.as_array()) {
            let mut out = String::new();
            for item in content {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    out.push_str(text);
                }
            }
            Ok(out)
        } else {
            Ok(serde_json::to_string_pretty(&res)?)
        }
    }
}
