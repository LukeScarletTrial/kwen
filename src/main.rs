mod config;
mod lmstudio;
mod tools;
mod mcp;
mod agent;
mod skills;
mod ui;
mod completer;

use clap::{Parser, Subcommand};
use rustyline::Editor;
use anyhow::Result;
use std::io::{self, Write};

use config::Config;
use tools::{ToolRegistry, ReadFileTool, WriteFileTool, EditFileTool, ListDirTool, ExecuteCommandTool, GrepSearchTool, FindFilesTool};
use mcp::{McpManager, McpServerConfig, get_curated_mcp_library};
use agent::Agent;
use skills::SkillManager;
use ui::Theme;
use completer::KwenHelper;

#[derive(Parser, Debug)]
#[command(name = "kwen", version = "0.1.0", about = "CLI Agent Harness tailored for small Qwen models with LM Studio & MCP support")]
struct Cli {
    #[arg(short, long, help = "Override LM Studio API base URL")]
    url: Option<String>,

    #[arg(short, long, help = "Override active model ID")]
    model: Option<String>,

    #[arg(short, long, help = "Override sampling temperature")]
    temperature: Option<f32>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start interactive REPL chat session (default)
    Chat,

    /// Execute a single one-shot instruction and exit
    Run {
        /// The instruction or prompt to execute
        prompt: String,
    },

    /// Manage MCP (Model Context Protocol) servers and addons
    Mcp {
        #[command(subcommand)]
        action: McpCommands,
    },

    /// List active tools available to the model
    Tools,

    /// List active skills
    Skills,

    /// Check connection and list available models in LM Studio
    Check,

    /// Show path to config.json and current settings
    Config,
}

#[derive(Subcommand, Debug)]
enum McpCommands {
    List,
    Library,
    Add {
        name: String,
        command: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Install { id: String },
    Remove { name: String },
}

async fn build_tools_with_mcp() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(ReadFileTool));
    registry.register(Box::new(WriteFileTool));
    registry.register(Box::new(EditFileTool));
    registry.register(Box::new(ListDirTool));
    registry.register(Box::new(ExecuteCommandTool));
    registry.register(Box::new(GrepSearchTool));
    registry.register(Box::new(FindFilesTool));
    if let Ok(manager) = McpManager::load_or_init() {
        manager.load_active_mcp_tools(&mut registry).await;
    }
    registry
}

/// Interactive first-time setup wizard. Returns the configured Config.
async fn run_setup_wizard(mut config: Config) -> Result<Config> {
    // Clear screen for wizard
    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;

    println!();
    println!("  \x1b[38;2;16;185;129m╭─────────────────────────────────────────────────────────────╮\x1b[0m");
    println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m\x1b[1mWelcome to Kwen! — First-Time Setup\x1b[0m                       \x1b[38;2;16;185;129m│\x1b[0m");
    println!("  \x1b[38;2;16;185;129m│\x1b[0m  This wizard configures your LM Studio connection.           \x1b[38;2;16;185;129m│\x1b[0m");
    println!("  \x1b[38;2;16;185;129m│\x1b[0m  Press \x1b[38;2;52;211;153mEnter\x1b[0m to accept defaults shown in \x1b[2m[brackets]\x1b[0m.        \x1b[38;2;16;185;129m│\x1b[0m");
    println!("  \x1b[38;2;16;185;129m╰─────────────────────────────────────────────────────────────╯\x1b[0m");
    println!();

    // Step 1: LM Studio URL
    let default_url = config.base_url.clone();
    print!("  \x1b[38;2;52;211;153m[1/4]\x1b[0m LM Studio API URL \x1b[2m[{}]\x1b[0m: ", default_url);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if !trimmed.is_empty() {
        config.base_url = trimmed.to_string();
    }

    // Test connectivity and fetch models
    println!();
    print!("  \x1b[38;2;52;211;153m⚙\x1b[0m  Connecting to LM Studio at \x1b[38;2;52;211;153m{}\x1b[0m ...", config.base_url);
    io::stdout().flush()?;

    let temp_tools = build_tools_with_mcp().await;
    let temp_agent = Agent::new(config.clone(), temp_tools);
    let available_models = match temp_agent.check_lmstudio_health().await {
        Ok(models) => {
            println!("  \x1b[38;2;52;211;153m[OK]\x1b[0m Found {} model(s).", models.len());
            models
        }
        Err(e) => {
            println!("  \x1b[38;2;245;158;11mWarning:\x1b[0m Could not connect ({})", e);
            println!("  \x1b[2mYou can configure a model manually later.\x1b[0m");
            vec![]
        }
    };

    // Step 2: Choose model
    println!();
    if !available_models.is_empty() {
        println!("  \x1b[38;2;52;211;153m[2/4]\x1b[0m Available models:");
        for (i, m) in available_models.iter().enumerate() {
            println!("       \x1b[38;2;16;185;129m{:2}.\x1b[0m {}", i + 1, m);
        }
        print!("  Enter number or model ID \x1b[2m[1 = {}]\x1b[0m: ", available_models[0]);
        io::stdout().flush()?;
        let mut model_input = String::new();
        io::stdin().read_line(&mut model_input)?;
        let model_trimmed = model_input.trim();
        if model_trimmed.is_empty() {
            config.model = available_models[0].clone();
        } else if let Ok(idx) = model_trimmed.parse::<usize>() {
            if idx >= 1 && idx <= available_models.len() {
                config.model = available_models[idx - 1].clone();
            } else {
                config.model = model_trimmed.to_string();
            }
        } else {
            config.model = model_trimmed.to_string();
        }
    } else {
        println!("  \x1b[38;2;52;211;153m[2/4]\x1b[0m Active model ID \x1b[2m[{}]\x1b[0m: ", config.model);
        io::stdout().flush()?;
        let mut model_input = String::new();
        io::stdin().read_line(&mut model_input)?;
        let m = model_input.trim();
        if !m.is_empty() {
            config.model = m.to_string();
        }
    }
    println!("        \x1b[38;2;52;211;153m✓\x1b[0m Model: \x1b[1m{}\x1b[0m", config.model);

    // Step 3: Temperature
    println!();
    print!("  \x1b[38;2;52;211;153m[3/4]\x1b[0m Sampling temperature \x1b[2m[{:.2}]\x1b[0m (0.0 = precise, 1.0 = creative): ", config.temperature);
    io::stdout().flush()?;
    let mut temp_input = String::new();
    io::stdin().read_line(&mut temp_input)?;
    let temp_str = temp_input.trim();
    if !temp_str.is_empty() {
        if let Ok(t) = temp_str.parse::<f32>() {
            config.temperature = t.clamp(0.0, 2.0);
        }
    }
    println!("        \x1b[38;2;52;211;153m✓\x1b[0m Temperature: \x1b[1m{:.2}\x1b[0m", config.temperature);

    // Step 4: Default context (optional)
    println!();
    print!("  \x1b[38;2;52;211;153m[4/4]\x1b[0m Default system context \x1b[2m[press Enter to skip]\x1b[0m: ");
    io::stdout().flush()?;
    let mut ctx_input = String::new();
    io::stdin().read_line(&mut ctx_input)?;
    let ctx_str = ctx_input.trim();
    if !ctx_str.is_empty() {
        config.default_context = Some(ctx_str.to_string());
    }

    // Save
    config.save()?;
    let cfg_path = Config::config_path();

    println!();
    println!("  \x1b[38;2;16;185;129m╭─────────────────────────────────────────────────────────────╮\x1b[0m");
    println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m✓ Setup complete!\x1b[0m Config saved to:                        \x1b[38;2;16;185;129m│\x1b[0m");
    println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[2m{:<59}\x1b[0m\x1b[38;2;16;185;129m│\x1b[0m", cfg_path.display());
    println!("  \x1b[38;2;16;185;129m╰─────────────────────────────────────────────────────────────╯\x1b[0m");
    println!();
    println!("  \x1b[38;2;52;211;153mStarting Kwen...\x1b[0m");
    println!();

    std::thread::sleep(std::time::Duration::from_millis(1000));
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config.json (returns (config, is_first_run))
    let (mut config, is_first_run) = Config::load();

    // CLI flags override config.json values
    if let Some(url) = cli.url { config.base_url = url; }
    if let Some(m) = cli.model { config.model = m; }
    if let Some(t) = cli.temperature { config.temperature = t; }

    match cli.command {
        Some(Commands::Config) => {
            Config::print_path();
            Theme::print_settings(&config);
            return Ok(());
        }

        Some(Commands::Tools) => {
            let tools = build_tools_with_mcp().await;
            println!("\n  \x1b[38;2;52;211;153m\x1b[1mActive Tools\x1b[0m");
            for def in tools.get_definitions() {
                println!("  \x1b[38;2;16;185;129m•\x1b[0m \x1b[1m{}\x1b[0m — {}", def.function.name, def.function.description);
            }
            println!();
            return Ok(());
        }

        Some(Commands::Skills) => {
            let skill_mgr = SkillManager::load_or_init()?;
            println!("\n  \x1b[38;2;52;211;153m\x1b[1mAvailable Skills\x1b[0m");
            for (name, skill) in &skill_mgr.skills {
                println!("  \x1b[38;2;16;185;129m•\x1b[0m \x1b[1m{}\x1b[0m — {}", name, skill.description);
            }
            println!();
            return Ok(());
        }

        Some(Commands::Check) => {
            let tools = build_tools_with_mcp().await;
            let agent = Agent::new(config.clone(), tools);
            match agent.check_lmstudio_health().await {
                Ok(models) => {
                    println!("\x1b[38;2;52;211;153m[OK]\x1b[0m Connected to LM Studio at {}", config.base_url);
                    for m in &models {
                        let marker = if m == &config.model { " \x1b[38;2;52;211;153m← active\x1b[0m" } else { "" };
                        println!("    {}{}", m, marker);
                    }
                }
                Err(e) => eprintln!("\x1b[38;2;239;68;68m[ERROR]\x1b[0m {}", e),
            }
            return Ok(());
        }

        Some(Commands::Mcp { action }) => {
            handle_mcp_command(action)?;
            return Ok(());
        }

        Some(Commands::Run { prompt }) => {
            let tools = build_tools_with_mcp().await;
            let mut agent = Agent::new(config, tools);
            println!("\x1b[38;2;52;211;153m>>\x1b[0m {}", prompt);
            match agent.process_user_turn(&prompt).await {
                Ok(reply) => { let _ = reply; }
                Err(e) => eprintln!("\x1b[38;2;239;68;68m[ERROR]\x1b[0m {}", e),
            }
            return Ok(());
        }

        Some(Commands::Chat) | None => {
            // Run first-time setup wizard if config didn't exist
            if is_first_run {
                config = run_setup_wizard(config).await?;
            }
            run_repl(config).await?;
        }
    }

    Ok(())
}

fn handle_mcp_command(action: McpCommands) -> Result<()> {
    let mut manager = McpManager::load_or_init()?;
    match action {
        McpCommands::List => {
            println!("\n  \x1b[38;2;52;211;153m\x1b[1mConfigured MCP Servers\x1b[0m");
            let servers = manager.list_servers();
            if servers.is_empty() {
                println!("  No MCP servers configured. Run \x1b[38;2;52;211;153mkwen mcp library\x1b[0m to browse.");
            } else {
                for (name, conf) in servers {
                    println!("  \x1b[38;2;16;185;129m•\x1b[0m \x1b[1m{}\x1b[0m → {} {}", name, conf.command, conf.args.join(" "));
                }
            }
            println!();
        }
        McpCommands::Library => {
            println!("\n  \x1b[38;2;52;211;153m\x1b[1mCurated MCP Add-Ons Library\x1b[0m");
            for item in get_curated_mcp_library() {
                println!("  \x1b[38;2;16;185;129m{:<12}\x1b[0m {} — {}", item.id, item.name, item.description);
                println!("               Install: \x1b[38;2;52;211;153mkwen mcp install {}\x1b[0m", item.id);
            }
            println!();
        }
        McpCommands::Install { id } => {
            let catalog = get_curated_mcp_library();
            if let Some(item) = catalog.iter().find(|i| i.id == id) {
                let conf = McpServerConfig {
                    command: item.command.to_string(),
                    args: item.default_args.iter().map(|s| s.to_string()).collect(),
                    env: std::collections::HashMap::new(),
                };
                manager.add_server(&item.id, conf)?;
                println!("\x1b[38;2;52;211;153m[OK]\x1b[0m Installed '{}'", item.name);
            } else {
                eprintln!("\x1b[38;2;239;68;68m[ERROR]\x1b[0m Unknown ID '{}'. Run 'kwen mcp library'.", id);
            }
        }
        McpCommands::Add { name, command, args } => {
            let conf = McpServerConfig { command, args, env: std::collections::HashMap::new() };
            manager.add_server(&name, conf)?;
            println!("\x1b[38;2;52;211;153m[OK]\x1b[0m Added MCP server '{}'", name);
        }
        McpCommands::Remove { name } => {
            if manager.remove_server(&name)? {
                println!("\x1b[38;2;52;211;153m[OK]\x1b[0m Removed '{}'", name);
            } else {
                eprintln!("\x1b[38;2;239;68;68m[ERROR]\x1b[0m Server '{}' not found.", name);
            }
        }
    }
    Ok(())
}

async fn run_repl(mut config: Config) -> Result<()> {
    // --- Take over the terminal: clear screen and move to top ---
    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;

    // ── Auto-detect loaded model ─────────────────────────────────────────────
    // Before showing banner, probe LM Studio and auto-select the loaded model
    // if the config model isn't available.
    {
        let temp_tools = build_tools_with_mcp().await;
        let temp_agent = Agent::new(config.clone(), temp_tools);
        if let Ok(models) = temp_agent.check_lmstudio_health().await {
            if !models.is_empty() && !models.contains(&config.model) {
                let old_model = config.model.clone();
                config.model = models[0].clone();
                // Print a brief notice (before banner)
                println!("  \x1b[38;2;245;158;11mNotice:\x1b[0m Model '\x1b[2m{}\x1b[0m' not found in LM Studio.", old_model);
                println!("          Auto-selected: \x1b[38;2;52;211;153m{}\x1b[0m", config.model);
                println!();
            }
        }
    }

    Theme::banner(&config);

    let tools = build_tools_with_mcp().await;
    let mut agent = Agent::new(config.clone(), tools);
    let skill_mgr = SkillManager::load_or_init()?;

    // Inject default_context from config into session if set
    if let Some(ref ctx) = config.default_context {
        if let Some(sys_msg) = agent.history.first_mut() {
            let curr = sys_msg.content.get_or_insert_with(String::new);
            curr.push_str(&format!("\n\n[Default Context from config.json]\n{}", ctx));
        }
    }

    match agent.check_lmstudio_health().await {
        Ok(models) => println!("  \x1b[38;2;52;211;153m[OK]\x1b[0m Connected to LM Studio! ({} models loaded)", models.len()),
        Err(e) => println!("  \x1b[38;2;245;158;11mNotice:\x1b[0m {}", e),
    }
    println!();

    let mut rl = Editor::new()?;
    let skill_names: Vec<String> = skill_mgr.skills.keys().cloned().collect();
    rl.set_helper(Some(KwenHelper::new(skill_names)));

    let history_file = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kwen").join("history.txt");
    let _ = rl.load_history(&history_file);

    loop {
        // Plain ASCII prompt — avoids rustyline cursor width calculation bugs with ANSI codes in prompt
        let readline = rl.readline("kwen> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                let _ = rl.add_history_entry(trimmed);

                // ── Slash command dispatch ────────────────────────────────────
                // Note: exact matches BEFORE prefix matches so "/model" alone
                // doesn't fall through to the AI.
                match trimmed {
                    "/exit" | "exit" | "/quit" | "quit" => {
                        println!("\n  \x1b[38;2;52;211;153mGoodbye! Happy coding with Kwen.\x1b[0m\n");
                        break;
                    }

                    "/help" => {
                        println!();
                        println!("  \x1b[38;2;16;185;129m╭── Slash Commands ────────────────────────────────────────────╮\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/context\x1b[0m       Token usage, message breakdown & tool stats  \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/settings\x1b[0m      View & adjust model, temperature, tokens     \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/skills\x1b[0m        List available specialized skills             \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/skill <name>\x1b[0m  Activate a skill in this session              \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/tools\x1b[0m         List active built-in and MCP tools            \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/clear\x1b[0m         Reset conversation history                    \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/model\x1b[0m         Show current model                            \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/model <id>\x1b[0m    Switch active model                           \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/models\x1b[0m        List all loaded LM Studio models              \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/set <k> <v>\x1b[0m   Set temp or model (e.g. /set temp 0.1)       \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m│\x1b[0m  \x1b[38;2;52;211;153m/exit\x1b[0m          Quit Kwen                                     \x1b[38;2;16;185;129m│\x1b[0m");
                        println!("  \x1b[38;2;16;185;129m╰──────────────────────────────────────────────────────────────╯\x1b[0m");
                        println!("  Shortcuts: \x1b[38;2;52;211;153mTab\x1b[0m complete • \x1b[38;2;52;211;153mCtrl+C\x1b[0m cancel line • \x1b[38;2;52;211;153mCtrl+D\x1b[0m quit\n");
                    }

                    "/context" => Theme::print_context_status(&agent),
                    "/settings" => Theme::print_settings(&agent.config),

                    "/tools" => {
                        println!();
                        println!("  \x1b[38;2;52;211;153m\x1b[1mActive Tools\x1b[0m");
                        for name in agent.tools.list_tool_names() {
                            println!("  \x1b[38;2;16;185;129m•\x1b[0m {}", name);
                        }
                        println!();
                    }

                    "/clear" => {
                        agent.history.clear();
                        agent.history.push(crate::lmstudio::ChatMessage {
                            role: "system".to_string(),
                            content: Some(Agent::default_qwen_system_prompt()),
                            tool_calls: None,
                            tool_call_id: None,
                            name: None,
                        });
                        println!("  \x1b[38;2;245;158;11mConversation history cleared.\x1b[0m");
                    }

                    "/skills" => {
                        println!();
                        println!("  \x1b[38;2;52;211;153m\x1b[1mAvailable Skills\x1b[0m");
                        for (name, s) in &skill_mgr.skills {
                            println!("  \x1b[38;2;16;185;129m•\x1b[0m \x1b[1m{:<18}\x1b[0m {}", name, s.description);
                        }
                        println!("  Activate: \x1b[38;2;52;211;153m/skill <name>\x1b[0m\n");
                    }

                    // /skill with no argument — show usage
                    "/skill" => {
                        println!("  \x1b[38;2;245;158;11mUsage:\x1b[0m /skill <name>  —  activate a skill by name.");
                        println!("  Run \x1b[38;2;52;211;153m/skills\x1b[0m to list all available skills.");
                    }

                    // /model with no argument — show current model and loaded list
                    "/model" => {
                        println!("  Current model: \x1b[38;2;52;211;153m{}\x1b[0m", agent.config.model);
                        println!("  Usage: \x1b[2m/model <id>\x1b[0m to switch, or \x1b[2m/models\x1b[0m to list loaded models.");
                    }

                    // /models — list all loaded models from LM Studio
                    "/models" => {
                        print!("  \x1b[2mFetching models from LM Studio...\x1b[0m");
                        io::stdout().flush().ok();
                        match agent.check_lmstudio_health().await {
                            Ok(models) => {
                                print!("\r                                        \r");
                                println!("  \x1b[38;2;52;211;153m\x1b[1mLoaded Models\x1b[0m ({} total):", models.len());
                                for m in &models {
                                    let active = if m == &agent.config.model { " \x1b[38;2;52;211;153m← active\x1b[0m" } else { "" };
                                    println!("  \x1b[38;2;16;185;129m•\x1b[0m {}{}", m, active);
                                }
                                println!("  Switch: \x1b[2m/model <id>\x1b[0m\n");
                            }
                            Err(e) => {
                                print!("\r                                        \r");
                                println!("  \x1b[38;2;239;68;68m[ERROR]\x1b[0m {}", e);
                            }
                        }
                    }

                    // /set with no argument — show usage
                    "/set" => {
                        println!("  \x1b[38;2;245;158;11mUsage:\x1b[0m");
                        println!("    /set temp <value>    — e.g. /set temp 0.1");
                        println!("    /set model <id>      — e.g. /set model qwen2.5-coder-7b-instruct");
                    }

                    s if s.starts_with("/skill ") => {
                        let s_name = s[7..].trim();
                        if s_name.is_empty() {
                            println!("  \x1b[38;2;245;158;11mUsage:\x1b[0m /skill <name>. Run /skills to list.");
                        } else if let Some(skill) = skill_mgr.skills.get(s_name) {
                            if let Some(sys_msg) = agent.history.first_mut() {
                                sys_msg.content.get_or_insert_with(String::new)
                                    .push_str(&format!("\n\n[Active Skill: {}]\n{}", skill.name, skill.prompt));
                            }
                            println!("  \x1b[38;2;52;211;153m[OK]\x1b[0m Activated skill: \x1b[1m{}\x1b[0m", skill.name);
                        } else {
                            println!("  \x1b[38;2;239;68;68m[ERROR]\x1b[0m Skill '{}' not found. Try /skills", s_name);
                        }
                    }

                    s if s.starts_with("/set ") => {
                        let parts: Vec<&str> = s[5..].split_whitespace().collect();
                        if parts.len() >= 2 {
                            match parts[0] {
                                "temp" | "temperature" => {
                                    if let Ok(t) = parts[1].parse::<f32>() {
                                        agent.config.temperature = t;
                                        println!("  \x1b[38;2;52;211;153m[OK]\x1b[0m Temperature → {}", t);
                                    } else {
                                        println!("  \x1b[38;2;239;68;68m[ERROR]\x1b[0m Invalid float");
                                    }
                                }
                                "model" => {
                                    agent.config.model = parts[1].to_string();
                                    println!("  \x1b[38;2;52;211;153m[OK]\x1b[0m Model → {}", parts[1]);
                                }
                                _ => println!("  Unknown key '{}'. Available: temp, model", parts[0]),
                            }
                        } else {
                            println!("  Usage: /set temp <value> | /set model <id>");
                        }
                    }

                    s if s.starts_with("/model ") => {
                        let new_model = s[7..].trim();
                        if new_model.is_empty() {
                            println!("  Current model: \x1b[38;2;52;211;153m{}\x1b[0m", agent.config.model);
                        } else {
                            agent.config.model = new_model.to_string();
                            println!("  \x1b[38;2;52;211;153m[OK]\x1b[0m Model → {}", new_model);
                        }
                    }

                    // Unknown slash command — show hint, don't send to AI
                    s if s.starts_with('/') => {
                        println!("  \x1b[38;2;245;158;11mUnknown command:\x1b[0m '{}'. Type \x1b[38;2;52;211;153m/help\x1b[0m for all commands.", s);
                    }

                    _ => {
                        match agent.process_user_turn(trimmed).await {
                            Ok(reply) => { let _ = reply; }
                            Err(e) => eprintln!("\n  \x1b[38;2;239;68;68m[ERROR]\x1b[0m {}\n", e),
                        }
                    }
                }
            }

            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("  ^C");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("\n  \x1b[38;2;52;211;153mGoodbye! Happy coding with Kwen.\x1b[0m\n");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_file);
    Ok(())
}
