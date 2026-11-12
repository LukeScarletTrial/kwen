use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use anyhow::{anyhow, Result};
use crate::tools::Tool;

pub struct ExecuteCommandTool;

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn name(&self) -> &str {
        "execute_command"
    }

    fn description(&self) -> &str {
        "Run a terminal command in the shell and return its stdout, stderr, and exit code."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command line string to run"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let cmd_str = arguments.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command' argument"))?;

        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("powershell");
            c.arg("-NoProfile").arg("-Command").arg(cmd_str);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.arg("-c").arg(cmd_str);
            c
        };

        let output = match cmd.output().await {
            Ok(out) => out,
            Err(e) => return Ok(format!("Failed to execute command: {}", e)),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);

        let mut res = format!("Exit code: {}\n", code);
        if !stdout.is_empty() {
            res.push_str("Stdout:\n");
            res.push_str(&stdout);
        }
        if !stderr.is_empty() {
            res.push_str("Stderr:\n");
            res.push_str(&stderr);
        }

        // Limit maximum output size for smaller LLM context windows
        if res.len() > 10000 {
            let truncated = &res[..10000];
            return Ok(format!("{}\n...[Output truncated to 10,000 characters]", truncated));
        }

        Ok(res)
    }
}
