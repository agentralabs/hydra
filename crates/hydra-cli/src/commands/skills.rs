use crate::client::HydraClient;
use crate::colors;
use crate::output;

struct SkillInfo {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    installed: bool,
}

fn builtin_skills() -> Vec<SkillInfo> {
    vec![
        SkillInfo {
            name: "code-review",
            version: "1.0.0",
            description: "Automated code review with sister-backed analysis",
            installed: true,
        },
        SkillInfo {
            name: "refactor",
            version: "1.0.0",
            description: "Safe refactoring with impact prediction",
            installed: true,
        },
        SkillInfo {
            name: "test-gen",
            version: "0.9.0",
            description: "Generate tests from code analysis",
            installed: true,
        },
        SkillInfo {
            name: "deploy",
            version: "0.8.0",
            description: "Multi-environment deployment orchestration",
            installed: false,
        },
        SkillInfo {
            name: "migrate-db",
            version: "0.7.0",
            description: "Database migration planning and execution",
            installed: false,
        },
        SkillInfo {
            name: "security-audit",
            version: "1.1.0",
            description: "Security vulnerability scanning and remediation",
            installed: false,
        },
    ]
}

pub fn list() {
    let client = HydraClient::new();
    if client.health_check() {
        output::print_info("Server connected");
    } else {
        output::print_warning("Hydra server unreachable. Showing locally known skills (offline fallback).");
    }

    output::print_header("Installed Skills");
    println!();

    let skills = builtin_skills();
    let installed: Vec<&SkillInfo> = skills.iter().filter(|s| s.installed).collect();

    if installed.is_empty() {
        output::print_dimmed("No skills installed. Use 'hydra skills install <name>' to add one.");
        return;
    }

    let headers = &["Skill", "Version", "Description"];
    let rows: Vec<Vec<String>> = installed
        .iter()
        .map(|s| {
            vec![
                s.name.to_string(),
                s.version.to_string(),
                s.description.to_string(),
            ]
        })
        .collect();
    output::print_table(headers, &rows);
    println!();
}

pub fn install(name: &str) {
    let skills = builtin_skills();
    if !skills.iter().any(|s| s.name == name) {
        output::print_error(&format!("Skill '{}' not found", name));
        output::print_info("Use 'hydra skills search <query>' to find skills");
        return;
    }

    let client = HydraClient::new();
    match client.post(
        &format!("/api/skills/{}/install", name),
        &serde_json::json!({}),
    ) {
        Ok(_) => {
            output::print_success(&format!("Installed skill: {}", colors::bold(name)));
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Skill '{}' was NOT installed.",
                name
            ));
        }
    }
}

pub fn remove(name: &str) {
    let skills = builtin_skills();
    if !skills.iter().any(|s| s.name == name) {
        output::print_error(&format!("Skill '{}' not found", name));
        return;
    }

    let client = HydraClient::new();
    match client.post(
        &format!("/api/skills/{}/remove", name),
        &serde_json::json!({}),
    ) {
        Ok(_) => {
            output::print_success(&format!("Removed skill: {}", colors::bold(name)));
        }
        Err(_) => {
            output::print_error(&format!(
                "Could not reach Hydra server. Skill '{}' was NOT removed.",
                name
            ));
        }
    }
}

pub fn search(query: &str) {
    let client = HydraClient::new();
    if !client.health_check() {
        output::print_warning("Hydra server unreachable. Showing locally known skills (offline fallback).");
    }

    output::print_header(&format!("Search: {}", query));
    println!();

    let skills = builtin_skills();
    let matches: Vec<&SkillInfo> = skills
        .iter()
        .filter(|s| {
            s.name.contains(query)
                || s.description.to_lowercase().contains(&query.to_lowercase())
        })
        .collect();

    if matches.is_empty() {
        output::print_dimmed(&format!("No skills matching '{}'", query));
        return;
    }

    let headers = &["Skill", "Version", "Status", "Description"];
    let rows: Vec<Vec<String>> = matches
        .iter()
        .map(|s| {
            let status = if s.installed {
                colors::green("installed")
            } else {
                colors::dim("available")
            };
            vec![
                s.name.to_string(),
                s.version.to_string(),
                status,
                s.description.to_string(),
            ]
        })
        .collect();
    output::print_table(headers, &rows);
    println!();
}
