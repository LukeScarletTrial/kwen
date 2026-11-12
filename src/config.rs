use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub default_context: Option<String>,
    /// True if this config was just created for the first time (not persisted yet as "setup done")
    #[serde(default)]
    pub first_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:1234/v1".to_string(),
            model: "qwen2.5-coder-7b-instruct".to_string(),
            temperature: 0.2,
            max_tokens: Some(4096),
            system_prompt: None,
            default_context: None,
            first_run: true,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kwen")
            .join("config.json")
    }

    /// Returns (config, is_first_run). is_first_run is true if no config.json existed.
    pub fn load() -> (Self, bool) {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<Config>(&data) {
                    cfg.first_run = false;
                    return (cfg, false);
                }
            }
        }
        // First run — create default but do NOT save yet (setup wizard will save)
        let default = Config { first_run: true, ..Config::default() };
        (default, true)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // Never persist first_run = true
        let mut to_save = self.clone();
        to_save.first_run = false;
        let json = serde_json::to_string_pretty(&to_save)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn print_path() {
        println!("  Config file: {}", Self::config_path().display());
    }
}
