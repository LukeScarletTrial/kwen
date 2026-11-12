use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub file_path: PathBuf,
}

pub struct SkillManager {
    pub skills: HashMap<String, Skill>,
    skills_dir: PathBuf,
}

impl SkillManager {
    pub fn load_or_init() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kwen")
            .join("skills");

        if !base_dir.exists() {
            let _ = fs::create_dir_all(&base_dir);
            Self::create_default_skills(&base_dir)?;
        }

        let mut manager = Self {
            skills: HashMap::new(),
            skills_dir: base_dir,
        };

        manager.reload_skills()?;
        Ok(manager)
    }

    fn create_default_skills(dir: &Path) -> Result<()> {
        let default_skills = vec![
            (
                "code-reviewer",
                "Perform an in-depth code review focusing on correctness, security, performance, and best practices for small models.",
                r#"# Skill: Code Reviewer
When reviewing code:
1. Examine code logic, corner cases, error handling, and potential panics or crashes.
2. Check for security vulnerabilities or unvalidated inputs.
3. Verify modularity, readability, and idiomatic patterns.
4. Provide structured feedback: Summary, Critical Issues, Improvements, and Concrete Code Suggestions.
"#
            ),
            (
                "bug-hunter",
                "Trace and debug elusive runtime errors, crashes, or edge-case failures.",
                r#"# Skill: Bug Hunter
When hunting bugs:
1. Read relevant code around the stacktrace or reported failure point.
2. Formulate 2-3 specific hypotheses about the root cause.
3. Inspect and verify variables, state transitions, and edge cases (empty strings, nulls, out of bounds).
4. Propose a minimal surgical fix that directly resolves the issue without introducing regressions.
"#
            ),
            (
                "rust-optimizer",
                "Optimize Rust code for memory efficiency, zero-cost abstractions, and concurrency.",
                r#"# Skill: Rust Optimizer
Focus on:
1. Eliminating unnecessary heap allocations (e.g. redundant `.clone()`, `to_string()`, unnecessary `Vec` copies).
2. Leveraging lifetimes, slices (`&str`, `&[T]`), and borrow semantics.
3. Ensuring async routines and locks (`tokio::sync::Mutex` vs `std::sync`) avoid deadlocks and contention.
4. Utilizing compiler optimizations and vectorization hints where applicable.
"#
            ),
            (
                "unit-tester",
                "Generate comprehensive unit and integration tests with edge-case coverage.",
                r#"# Skill: Unit Tester
When writing tests:
1. Cover standard happy paths and expected outcomes.
2. Thoroughly test boundary conditions (zero, empty, max limits, special characters).
3. Test failure and error states (invalid inputs, network timeouts, bad JSON).
4. Keep tests self-contained and clean.
"#
            ),
        ];

        for (name, desc, prompt) in default_skills {
            let file_path = dir.join(format!("{}.md", name));
            let content = format!("---\nname: {}\ndescription: {}\n---\n\n{}", name, desc, prompt);
            let _ = fs::write(file_path, content);
        }

        Ok(())
    }

    pub fn reload_skills(&mut self) -> Result<()> {
        self.skills.clear();
        if !self.skills_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(skill) = Self::parse_skill(&path, &content) {
                        self.skills.insert(skill.name.clone(), skill);
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_skill(path: &Path, content: &str) -> Option<Skill> {
        let name_default = path.file_stem()?.to_string_lossy().to_string();
        let mut name = name_default.clone();
        let mut description = "Custom skill".to_string();
        let mut body = content.to_string();

        if content.starts_with("---") {
            if let Some(second_dash) = content[3..].find("---") {
                let header = &content[3..3 + second_dash];
                body = content[3 + second_dash + 3..].trim().to_string();

                for line in header.lines() {
                    let line = line.trim();
                    if line.starts_with("name:") {
                        name = line[5..].trim().to_string();
                    } else if line.starts_with("description:") {
                        description = line[12..].trim().to_string();
                    }
                }
            }
        }

        Some(Skill {
            name,
            description,
            prompt: body,
            file_path: path.to_path_buf(),
        })
    }
}
