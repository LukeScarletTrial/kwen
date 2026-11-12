use crate::config::Config;
use crate::lmstudio::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ModelsResponse};
use crate::tools::ToolRegistry;
use crate::ui::Theme;
use reqwest::Client;
use colored::*;
use anyhow::{anyhow, Result};
use std::time::{Duration, Instant};
use std::io::{self, Write};

pub struct Agent {
    pub config: Config,
    client: Client,
    pub tools: ToolRegistry,
    pub history: Vec<ChatMessage>,
}

impl Agent {
    pub fn new(config: Config, tools: ToolRegistry) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        let mut history = Vec::new();
        let sys_prompt = config.system_prompt.clone().unwrap_or_else(|| {
            Self::default_qwen_system_prompt()
        });

        history.push(ChatMessage {
            role: "system".to_string(),
            content: Some(sys_prompt),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        Self {
            config,
            client,
            tools,
            history,
        }
    }

    pub fn default_qwen_system_prompt() -> String {
        r#"You are Kwen, an expert autonomous CLI software engineering assistant tailored for Qwen models.
You have direct access to system tools to inspect, read, edit files, and execute shell commands.

GUIDELINES FOR RELIABLE OPERATION WITH SMALLER MODELS:
1. Always prefer using tools rather than speculating or guessing file contents.
2. When calling tools, strictly output valid JSON matching the schema parameters.
3. Be concise and precise. Explain your plan before making changes, then inspect your results.
4. When editing files, ensure the target string matches the exact existing code segment.
5. If a command or tool returns an error, carefully analyze the error and try a revised approach.
"#.to_string()
    }

    pub async fn check_lmstudio_health(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.config.base_url.trim_end_matches('/'));
        let res = self.client.get(&url).send().await
            .map_err(|e| anyhow!("Failed to connect to LM Studio at {}: {}. Ensure LM Studio local server is running!", self.config.base_url, e))?;

        if !res.status().is_success() {
            return Err(anyhow!("LM Studio server returned error status: {}", res.status()));
        }

        let models_resp: ModelsResponse = res.json().await
            .map_err(|e| anyhow!("Failed to parse models response from LM Studio: {}", e))?;

        Ok(models_resp.data.into_iter().map(|m| m.id).collect())
    }

    pub async fn process_user_turn(&mut self, user_input: &str) -> Result<String> {
        self.history.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_input.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        let mut round = 0;
        let max_rounds = 12;

        loop {
            round += 1;
            if round > max_rounds {
                let note = "Reached maximum tool call iterations for this turn.".to_string();
                println!("{}", note.yellow());
                return Ok(note);
            }

            let tool_defs = self.tools.get_definitions();
            let req = ChatCompletionRequest {
                model: self.config.model.clone(),
                messages: self.history.clone(),
                tools: if tool_defs.is_empty() { None } else { Some(tool_defs) },
                temperature: Some(self.config.temperature),
                max_tokens: self.config.max_tokens,
                stream: None,
            };

            let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
            Theme::prompt_processing();
            let start_time = Instant::now();

            let response = self.client.post(&url)
                .json(&req)
                .send()
                .await
                .map_err(|e| anyhow!("Error querying LM Studio at {}: {}", url, e))?;

            Theme::thinking_start();

            if !response.status().is_success() {
                Theme::thinking_done();
                let text = response.text().await.unwrap_or_default();
                return Err(anyhow!("LM Studio returned error: {}", text));
            }

            let mut completion: ChatCompletionResponse = response.json().await
                .map_err(|e| anyhow!("Failed to parse chat response: {}", e))?;

            Theme::thinking_done();
            let duration = start_time.elapsed().as_secs_f64();

            let choice = completion.choices.pop()
                .ok_or_else(|| anyhow!("No choices returned by LM Studio"))?;

            let assistant_msg = choice.message;
            self.history.push(assistant_msg.clone());

            // Check if there are tool calls
            if let Some(tool_calls) = &assistant_msg.tool_calls {
                if !tool_calls.is_empty() {
                    for tool_call in tool_calls {
                        let fn_name = &tool_call.function.name;
                        let fn_args = &tool_call.function.arguments;

                        Theme::tool_call(fn_name, fn_args);

                        let exec_res = match self.tools.execute(fn_name, fn_args).await {
                            Ok(output) => output,
                            Err(e) => format!("Error executing {}: {}", fn_name, e),
                        };

                        let preview = if exec_res.len() > 250 {
                            format!("{}... [truncated]", &exec_res[..250])
                        } else {
                            exec_res.clone()
                        };
                        Theme::tool_result(&preview);

                        // Append tool response
                        self.history.push(ChatMessage {
                            role: "tool".to_string(),
                            content: Some(exec_res),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                            name: Some(fn_name.clone()),
                        });
                    }

                    // Continue loop so model receives tool feedback
                    continue;
                }
            }

            // Also check for raw XML or tag-style tool calls commonly produced by Qwen models
            if let Some(ref text) = assistant_msg.content {
                if let Some((extracted_name, extracted_args)) = Self::try_parse_qwen_xml_tool_call(text) {
                    Theme::tool_call(&format!("{} (xml parsed)", extracted_name), &extracted_args);
                    let exec_res = match self.tools.execute(&extracted_name, &extracted_args).await {
                        Ok(output) => output,
                        Err(e) => format!("Error executing {}: {}", extracted_name, e),
                    };

                    let preview = if exec_res.len() > 250 {
                        format!("{}... [truncated]", &exec_res[..250])
                    } else {
                        exec_res.clone()
                    };
                    Theme::tool_result(&preview);

                    self.history.push(ChatMessage {
                        role: "user".to_string(),
                        content: Some(format!("[Tool Result for {}]:\n{}", extracted_name, exec_res)),
                        tool_calls: None,
                        tool_call_id: None,
                        name: Some(extracted_name),
                    });
                    continue;
                }

                // Normal response text - slowly animate / typewriter effect for AI text generation
                Self::typewriter_display(text);

                // Render Markdown block if text has codeblocks or headers
                if text.contains("```") || text.contains("###") || text.contains("- ") {
                    println!("\n{}", "── 📄 Rendered Markdown View ──────────────────────────────".dimmed());
                    Theme::render_markdown(text);
                }

                // Print token statistics and speed (t/s)
                let token_count = completion.usage
                    .and_then(|u| u.completion_tokens)
                    .unwrap_or_else(|| (text.len() / 4).max(1) as u32);

                Theme::print_stats(token_count, duration);

                return Ok(text.clone());
            }

            return Ok(String::new());
        }
    }

    fn typewriter_display(text: &str) {
        println!();
        let mut stdout = io::stdout();

        // Print chunks smoothly to simulate live token generation
        for ch in text.chars() {
            print!("{}", ch);
            let _ = stdout.flush();
            std::thread::sleep(Duration::from_millis(8));
        }
        println!();
    }

    fn try_parse_qwen_xml_tool_call(text: &str) -> Option<(String, String)> {
        if let Some(start) = text.find("<tool_call>") {
            if let Some(end) = text[start..].find("</tool_call>") {
                let inner = &text[start + 11..start + end].trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(inner) {
                    if let (Some(name), Some(args)) = (val.get("name").and_then(|n| n.as_str()), val.get("arguments")) {
                        let args_str = if args.is_string() {
                            args.as_str().unwrap().to_string()
                        } else {
                            args.to_string()
                        };
                        return Some((name.to_string(), args_str));
                    }
                }
            }
        }
        None
    }
}
