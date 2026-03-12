//! Skills and agent file loading (spec §13, §15).
//! Loads .md files from .hydra/skills/, ~/.hydra/skills/, .hydra/agents/, ~/.hydra/agents/

use std::path::{Path, PathBuf};

/// A loaded skill definition.
#[derive(Debug, Clone)]
pub struct SkillDef {
    pub name: String,
    pub description: String,
    pub allowed_tools: Vec<String>,
    pub argument_hint: String,
    pub auto_invoke: bool,
    pub body: String,
    pub source: SkillSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkillSource {
    Project,
    Personal,
}

/// A loaded agent definition.
#[derive(Debug, Clone)]
pub struct AgentDef {
    pub name: String,
    pub model: String,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub system_prompt: String,
    pub source: SkillSource,
}

/// Load all skills from project and personal directories.
pub fn load_skills() -> Vec<SkillDef> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut skills = Vec::new();

    // Project skills
    load_skills_from_dir(Path::new(".hydra/skills"), SkillSource::Project, &mut skills);

    // Personal skills
    let personal = PathBuf::from(&home).join(".hydra/skills");
    load_skills_from_dir(&personal, SkillSource::Personal, &mut skills);

    skills
}

fn load_skills_from_dir(dir: &Path, source: SkillSource, out: &mut Vec<SkillDef>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(skill) = parse_skill_file(&path, source.clone()) {
                out.push(skill);
            }
        }
    }
}

fn parse_skill_file(path: &Path, source: SkillSource) -> Option<SkillDef> {
    let content = std::fs::read_to_string(path).ok()?;
    let (frontmatter, body) = split_frontmatter(&content)?;

    let mut name = String::new();
    let mut description = String::new();
    let mut allowed_tools = Vec::new();
    let mut argument_hint = String::new();
    let mut auto_invoke = false;

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("description:") {
            description = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("allowed-tools:") {
            allowed_tools = val.split(',').map(|s| s.trim().to_string()).collect();
        } else if let Some(val) = line.strip_prefix("argument-hint:") {
            argument_hint = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("disable-model-invocation:") {
            auto_invoke = val.trim() == "false";
        }
    }

    if name.is_empty() {
        name = path.file_stem()?.to_string_lossy().to_string();
    }

    Some(SkillDef { name, description, allowed_tools, argument_hint, auto_invoke, body, source })
}

/// Load all agent definitions.
pub fn load_agents() -> Vec<AgentDef> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut agents = Vec::new();

    load_agents_from_dir(Path::new(".hydra/agents"), SkillSource::Project, &mut agents);
    let personal = PathBuf::from(&home).join(".hydra/agents");
    load_agents_from_dir(&personal, SkillSource::Personal, &mut agents);

    agents
}

fn load_agents_from_dir(dir: &Path, source: SkillSource, out: &mut Vec<AgentDef>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(agent) = parse_agent_file(&path, source.clone()) {
                out.push(agent);
            }
        }
    }
}

fn parse_agent_file(path: &Path, source: SkillSource) -> Option<AgentDef> {
    let content = std::fs::read_to_string(path).ok()?;
    let (frontmatter, body) = split_frontmatter(&content)?;

    let mut name = String::new();
    let mut model = "sonnet".to_string();
    let mut allowed_tools = Vec::new();
    let mut disallowed_tools = Vec::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("model:") {
            model = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("allowed-tools:") {
            allowed_tools = val.split(',').map(|s| s.trim().to_string()).collect();
        } else if let Some(val) = line.strip_prefix("disallowed-tools:") {
            disallowed_tools = val.split(',').map(|s| s.trim().to_string()).collect();
        }
    }

    if name.is_empty() {
        name = path.file_stem()?.to_string_lossy().to_string();
    }

    Some(AgentDef { name, model, allowed_tools, disallowed_tools, system_prompt: body, source })
}

/// Split markdown content into (frontmatter, body).
fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let content = content.trim_start();
    if !content.starts_with("---") { return Some((String::new(), content.to_string())); }
    let after_first = &content[3..];
    let end = after_first.find("---")?;
    let fm = after_first[..end].to_string();
    let body = after_first[end + 3..].trim().to_string();
    Some((fm, body))
}

/// Get skill names as slash command names (prefixed with /).
pub fn skill_command_names(skills: &[SkillDef]) -> Vec<String> {
    skills.iter().map(|s| format!("/{}", s.name)).collect()
}
