use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use regex::Regex;
use anyhow::{anyhow, Result};
use crate::tools::Tool;

pub struct GrepSearchTool;

#[async_trait]
impl Tool for GrepSearchTool {
    fn name(&self) -> &str {
        "grep_search"
    }

    fn description(&self) -> &str {
        "Search for a regex pattern in files across a directory."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression or substring to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: .)"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the search is case sensitive (default: false)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let pattern_str = arguments.get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'pattern' argument"))?;

        let root_path_str = arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let case_sensitive = arguments.get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let regex = if case_sensitive {
            Regex::new(pattern_str)
        } else {
            Regex::new(&format!("(?i){}", pattern_str))
        };

        let regex = match regex {
            Ok(r) => r,
            Err(e) => return Ok(format!("Invalid regex pattern: {}", e)),
        };

        let mut matches = Vec::new();
        let max_matches = 100;

        for entry in WalkDir::new(root_path_str)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "target" && name != ".git" && name != "node_modules"
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let p = entry.path();
                if let Ok(content) = fs::read_to_string(p) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            matches.push(format!("{}:{}: {}", p.display(), line_idx + 1, line.trim()));
                            if matches.len() >= max_matches {
                                matches.push("... (Maximum matches reached)".to_string());
                                return Ok(matches.join("\n"));
                            }
                        }
                    }
                }
            }
        }

        if matches.is_empty() {
            Ok(format!("No matches found for pattern '{}'", pattern_str))
        } else {
            Ok(matches.join("\n"))
        }
    }
}

pub struct FindFilesTool;

#[async_trait]
impl Tool for FindFilesTool {
    fn name(&self) -> &str {
        "find_files"
    }

    fn description(&self) -> &str {
        "Find files matching a glob or substring pattern in a directory tree."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Filename pattern to search for (e.g., '*.rs', 'config', or 'json')"
                },
                "path": {
                    "type": "string",
                    "description": "Root directory to search (default: .)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let pattern = arguments.get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'pattern' argument"))?;

        let root_str = arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let lower_pattern = pattern.to_lowercase();
        let mut results = Vec::new();

        for entry in WalkDir::new(root_str)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "target" && name != ".git" && name != "node_modules"
            })
            .filter_map(|e| e.ok())
        {
            let filename = entry.file_name().to_string_lossy().to_lowercase();
            if filename.contains(&lower_pattern) {
                results.push(entry.path().display().to_string());
                if results.len() >= 100 {
                    results.push("... (Maximum results reached)".to_string());
                    break;
                }
            }
        }

        if results.is_empty() {
            Ok(format!("No files found matching '{}'", pattern))
        } else {
            Ok(results.join("\n"))
        }
    }
}
