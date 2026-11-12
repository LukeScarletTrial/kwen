use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use anyhow::{anyhow, Result};
use crate::tools::Tool;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Supports optional start_line and end_line parameters."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read"
                },
                "start_line": {
                    "type": "integer",
                    "description": "Optional 1-based start line number to read"
                },
                "end_line": {
                    "type": "integer",
                    "description": "Optional 1-based end line number to read (inclusive)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let path_str = arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(format!("Error: File not found: {}", path_str));
        }

        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let start_line = arguments.get("start_line")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let end_line = arguments.get("end_line")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(lines.len());

        let start_idx = if start_line > 0 { start_line - 1 } else { 0 };
        let end_idx = end_line.min(lines.len());

        if start_idx >= lines.len() {
            return Ok(format!("File has {} lines. start_line {} is out of range.", lines.len(), start_line));
        }

        let mut output = String::new();
        for (i, line) in lines[start_idx..end_idx].iter().enumerate() {
            output.push_str(&format!("{:4} | {}\n", start_idx + i + 1, line));
        }

        Ok(output)
    }
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with the given content."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to create or overwrite"
                },
                "content": {
                    "type": "string",
                    "description": "The text content to write into the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let path_str = arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

        let content = arguments.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'content' argument"))?;

        let path = Path::new(path_str);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(path, content)?;
        Ok(format!("Successfully wrote {} bytes to {}", content.len(), path_str))
    }
}

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replace an exact string (target) with a new string (replacement) in a file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to modify"
                },
                "target": {
                    "type": "string",
                    "description": "The exact string segment to replace"
                },
                "replacement": {
                    "type": "string",
                    "description": "The new string segment to replace target with"
                }
            },
            "required": ["path", "target", "replacement"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let path_str = arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'path' argument"))?;

        let target = arguments.get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'target' argument"))?;

        let replacement = arguments.get("replacement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'replacement' argument"))?;

        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(format!("Error: File not found: {}", path_str));
        }

        let content = fs::read_to_string(path)?;
        if !content.contains(target) {
            return Ok(format!("Error: target string not found in {}", path_str));
        }

        let new_content = content.replacen(target, replacement, 1);
        fs::write(path, new_content)?;

        Ok(format!("Successfully applied replacement in {}", path_str))
    }
}

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List files and directories at a given path."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default is current directory .)"
                }
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let path_str = arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(format!("Error: Directory not found: {}", path_str));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let kind = if file_type.is_dir() { "DIR " } else { "FILE" };
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(format!("[{}] {}", kind, name));
        }

        entries.sort();
        Ok(entries.join("\n"))
    }
}
