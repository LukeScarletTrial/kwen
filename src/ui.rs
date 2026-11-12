use crate::config::Config;
use crate::agent::Agent;

// Emerald green hex: #10B981 = RGB(16, 185, 129)
// Mint green hex:    #34D399 = RGB(52, 211, 153)
// Pale mint:         #D1FAE5 = RGB(209, 250, 229)

// Directly embed 24-bit ANSI escape codes — bypasses `colored` truecolor detection
// so the correct RGB color is always rendered regardless of COLORTERM env var.
macro_rules! emerald {
    ($s:expr) => {
        format!("\x1b[38;2;16;185;129m{}\x1b[0m", $s)
    };
}

macro_rules! mint {
    ($s:expr) => {
        format!("\x1b[38;2;52;211;153m{}\x1b[0m", $s)
    };
}

macro_rules! pale {
    ($s:expr) => {
        format!("\x1b[38;2;209;250;229m{}\x1b[0m", $s)
    };
}

macro_rules! dim_green {
    ($s:expr) => {
        format!("\x1b[38;2;6;95;70m{}\x1b[0m", $s)
    };
}

pub struct Theme;

impl Theme {
    pub fn banner(config: &Config) {
        let kwen_logo = [
            r#"██╗  ██╗██╗    ██╗███████╗███╗   ██╗"#,
            r#"██║ ██╔╝██║    ██║██╔════╝████╗  ██║"#,
            r#"█████╔╝ ██║ █╗ ██║█████╗  ██╔██╗ ██║"#,
            r#"██╔═██╗ ██║███╗██║██╔══╝  ██║╚██╗██║"#,
            r#"██║  ██╗╚███╔███╔╝███████╗██║ ╚████║"#,
            r#"╚═╝  ╚═╝ ╚══╝╚══╝ ╚══════╝╚═╝  ╚═══╝"#,
        ];

        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let truncated_cwd = if cwd.len() > 34 {
            format!("...{}", &cwd[cwd.len() - 31..])
        } else {
            cwd
        };

        println!();
        println!("  {}", emerald!("╭──────────────────────────────────────┬──────────────────────────────────────────╮"));

        for i in 0..6 {
            let logo_part = format!("{:<36}", kwen_logo[i]);

            let info_raw = match i {
                0 => format!(">_ \x1b[1mKwen Agent\x1b[0m \x1b[2m(v0.1.0)\x1b[0m"),
                1 => format!("Qwen Optimized Harness | LM Studio"),
                2 => format!("Provider: \x1b[38;2;52;211;153mLM Studio (Local)\x1b[0m"),
                3 => format!("Model:    \x1b[38;2;167;243;208m{}\x1b[0m", config.model),
                4 => format!("Path:     \x1b[38;2;52;211;153m{}\x1b[0m", truncated_cwd),
                5 => format!("Temp:     \x1b[1m{:.2}\x1b[0m | Max Tokens: \x1b[1m{}\x1b[0m", config.temperature, config.max_tokens.unwrap_or(4096)),
                _ => String::new(),
            };

            // Logo in bold emerald (#10B981), info panel plain
            println!("  {}  \x1b[38;2;16;185;129m\x1b[1m{}\x1b[0m  {}  {:<40}  {}",
                emerald!("│"),
                logo_part,
                emerald!("│"),
                info_raw,
                emerald!("│"),
            );
        }

        println!("  {}", emerald!("╰──────────────────────────────────────┴──────────────────────────────────────────╯"));
        println!("  {}", dim_green!("Tips: Tab to auto-complete commands • Ctrl+C to cancel line • /exit to quit"));
        println!();
    }

    pub fn ok(msg: &str) {
        println!("  {} {}", mint!("[OK]"), pale!(msg));
    }

    pub fn notice(msg: &str) {
        println!("  {} {}", emerald!("Notice:"), pale!(msg));
    }

    pub fn error(msg: &str) {
        eprintln!("  \x1b[38;2;239;68;68m[ERROR]\x1b[0m {}", msg);
    }

    pub fn tool_call(name: &str, args: &str) {
        println!("  {} {} \x1b[2m{}\x1b[0m", mint!("⚡ Tool Call:"), emerald!(name), args);
    }

    pub fn tool_result(preview: &str) {
        println!("  {} \x1b[2m{}\x1b[0m", mint!("↳ Result:"), preview);
    }

    pub fn prompt_processing() {
        print!("  {} \x1b[2m{}\x1b[0m", emerald!("⚙"), "Processing prompt with LM Studio...");
    }

    pub fn thinking_start() {
        print!("\r  {} {}", mint!("●"), pale!("Thinking & generating response...    "));
    }

    pub fn thinking_done() {
        print!("\r                                                           \r");
    }

    pub fn print_stats(tokens: u32, duration_secs: f64) {
        let tps = if duration_secs > 0.0 { tokens as f64 / duration_secs } else { 0.0 };
        println!();
        println!("  {} \x1b[1m{:.2}s\x1b[0m {} {} {}",
            dim_green!("⏱"),
            duration_secs,
            emerald!("•"),
            mint!(&format!("{} tokens", tokens)),
            emerald!(&format!("({:.1} t/s)", tps))
        );
    }

    pub fn render_markdown(md_text: &str) {
        let skin = termimad::MadSkin::default();
        println!();
        skin.print_text(md_text);
    }

    pub fn print_context_status(agent: &Agent) {
        println!();
        println!("  {}", emerald!("╭── Context & Conversation Status ─────────────────────────────╮"));
        let total_msgs = agent.history.len();
        let (mut total_chars, mut system_chars, mut user_chars, mut assistant_chars, mut tool_chars) = (0, 0, 0, 0, 0);
        for msg in &agent.history {
            let len = msg.content.as_ref().map(|s| s.len()).unwrap_or(0);
            total_chars += len;
            match msg.role.as_str() {
                "system" => system_chars += len,
                "user" => user_chars += len,
                "assistant" => assistant_chars += len,
                "tool" => tool_chars += len,
                _ => {}
            }
        }
        let est_tokens = total_chars / 4;
        println!("  {} Total Messages: {:<45}{}", emerald!("│"), total_msgs, emerald!("│"));
        println!("  {} Est. Tokens:    {:<45}{}", emerald!("│"), mint!(&format!("~{} tokens ({} chars)", est_tokens, total_chars)), emerald!("│"));
        println!("  {}   • System:     {:<45}{}", emerald!("│"), format!("{} chars", system_chars), emerald!("│"));
        println!("  {}   • User:       {:<45}{}", emerald!("│"), format!("{} chars", user_chars), emerald!("│"));
        println!("  {}   • Assistant:  {:<45}{}", emerald!("│"), format!("{} chars", assistant_chars), emerald!("│"));
        println!("  {}   • Tools:      {:<45}{}", emerald!("│"), format!("{} chars", tool_chars), emerald!("│"));
        println!("  {} Active Tools:   {:<45}{}", emerald!("│"), emerald!(&agent.tools.list_tool_names().len().to_string()), emerald!("│"));
        println!("  {}", emerald!("╰──────────────────────────────────────────────────────────────╯"));
        println!();
    }

    pub fn print_settings(config: &Config) {
        println!();
        println!("  {}", emerald!("╭── Current Settings ──────────────────────────────────────────╮"));
        println!("  {} Base URL:       {:<45}{}", emerald!("│"), mint!(&config.base_url), emerald!("│"));
        println!("  {} Active Model:   {:<45}{}", emerald!("│"), pale!(&config.model), emerald!("│"));
        println!("  {} Temperature:    {:<45}{}", emerald!("│"), format!("{:.2}", config.temperature), emerald!("│"));
        println!("  {} Max Tokens:     {:<45}{}", emerald!("│"), config.max_tokens.map(|v| v.to_string()).unwrap_or_else(|| "unlimited".to_string()), emerald!("│"));
        println!("  {} System Prompt:  {:<45}{}", emerald!("│"), if config.system_prompt.is_some() { "Custom" } else { "Default Qwen Optimized" }, emerald!("│"));
        println!("  {}", emerald!("╰──────────────────────────────────────────────────────────────╯"));
        println!("  Quick commands: {} | {}", mint!("/set model <name>"), mint!("/set temp <value>"));
        println!();
    }
}
