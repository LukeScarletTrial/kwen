use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct KwenHelper {
    commands: Vec<(&'static str, &'static str)>,
}

impl KwenHelper {
    pub fn new(skills: Vec<String>) -> Self {
        let mut commands = vec![
            ("/help", "Show help and commands"),
            ("/context", "Inspect token usage, messages & stats"),
            ("/settings", "View and adjust model & temperature"),
            ("/skills", "List available skills"),
            ("/skill", "Activate a skill (e.g. /skill code-reviewer)"),
            ("/tools", "List active tools and MCP add-ons"),
            ("/clear", "Reset conversation history"),
            ("/model", "Show or switch active LLM model"),
            ("/models", "List all loaded LM Studio models"),
            ("/set", "Set config value (e.g. /set temp 0.2)"),
            ("/exit", "Exit Kwen session"),
        ];

        for s in skills {
            let s_static: &'static str = Box::leak(s.into_boxed_str());
            let full_cmd: &'static str = Box::leak(format!("/skill {}", s_static).into_boxed_str());
            commands.push((full_cmd, "Activate specialized skill prompt"));
        }

        Self { commands }
    }
}

impl Completer for KwenHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let current = &line[..pos];
        let mut candidates = Vec::new();

        if current.starts_with('/') {
            for (cmd, desc) in &self.commands {
                if cmd.starts_with(current) {
                    candidates.push(Pair {
                        display: format!("{:<22} {}", cmd, desc),
                        replacement: format!("{} ", cmd),
                    });
                }
            }
        }

        Ok((0, candidates))
    }
}

impl Hinter for KwenHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if line.is_empty() || pos < line.len() || !line.starts_with('/') {
            return None;
        }

        for (cmd, _) in &self.commands {
            if cmd.starts_with(line) && cmd.len() > line.len() {
                return Some(cmd[line.len()..].to_string());
            }
        }
        None
    }
}

impl Highlighter for KwenHelper {}
impl Validator for KwenHelper {}
impl Helper for KwenHelper {}
