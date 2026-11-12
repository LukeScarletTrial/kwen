# 🦀 Kwen Agent Harness

> **A fast, native Rust CLI agent harness inspired by Qwen Code & Claude Code, crafted specifically for smaller Qwen Models (Qwen 3.5 9B, Qwen 3 35B A3B, Qwen 2.5 Coder 7B/14B/32B), powered by LM Studio and extensible with MCP Add-Ons and Skills.**

---

## ✨ Features & Aesthetic

- **Qwen Code & Claude Code Inspired UI**:
  - Clean rounded framed banners, status cards, and live visual cues (`⚡` Tool Call, `↳` Result, `●` Thinking).
  - Elegant prompt (`kwen ❯ `) with **Tab auto-completion** and inline ghost suggestions for all `/` commands and skills.
- **Dedicated Slash Commands**:
  - `/context`: Visual breakdown of message history, character breakdown (system/user/assistant/tools), and estimated token count against small context limits.
  - `/settings`: View current LM Studio provider URL, active model, sampling temperature, max tokens, and adjust them dynamically on the fly (`/set temp 0.1`, `/set model <name>`).
  - `/skills`: Browse installed specialized prompt engineering and workflow skills.
  - `/skill <name>`: Instantly inject specialized workflows (e.g. `code-reviewer`, `bug-hunter`, `rust-optimizer`, `unit-tester`).
  - `/tools`: Inspect active built-in tools and connected MCP Add-Ons.
  - `/clear`: Reset conversation context.
  - `/help`: Quick command reference card.
- **Skills System**:
  - Skills are stored in user config markdown files (`~/.config/kwen/skills/*.md`).
  - Edit or add any new custom skill in Markdown with YAML frontmatter:
    ```markdown
    ---
    name: sql-expert
    description: Writes optimized Postgres queries and indexes
    ---
    # Instructions here...
    ```
- **Built-in Tools**:
  - `read_file`, `write_file`, `edit_file`, `list_dir`, `execute_command`, `grep_search`, `find_files`.
- **MCP (Model Context Protocol) Add-Ons & Library**:
  - Curated Add-On library (`kwen mcp library`) with 1-click install (`kwen mcp install <id>`).
  - Supports SQLite, Fetch/Scraping, Git, Filesystem, GitHub, Memory, etc.

---

## 🚀 Quick Usage

### Start Interactive Chat REPL
```bash
kwen chat
# or with specific model
kwen --model "qwen3.5-9b" chat
```

### Try Slash Commands in REPL
- Press `Tab` on `/` to see all auto-completable commands.
- Run `/context` to check context size and message distribution.
- Run `/settings` to see and adjust runtime parameters.
- Run `/skill code-reviewer` to focus the agent on deep code review.
- Run `/set temp 0.1` to lock in lower sampling temperature.
- Run `/exit` or `Ctrl+C` to quit.

### MCP Management
```bash
kwen mcp library            # Browse curated addons
kwen mcp install fetch      # Install web scraping MCP
kwen mcp list               # View installed addons
```
