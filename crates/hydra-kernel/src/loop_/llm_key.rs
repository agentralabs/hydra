//! API key resolution — vault first, env var fallback, dotenv last resort.
//! Shared between LlmCaller (main calls) and micro_call (classification).

/// Resolve an API key: vault first → env var → empty string.
pub fn resolve_api_key(provider: &str, env_key: &str) -> String {
    if let Ok(key) = std::env::var(env_key) {
        if !key.is_empty() { return key; }
    }
    if let Some(key) = vault_get_api_key(provider) {
        eprintln!("hydra-llm: API key loaded from vault/{provider}.toml");
        return key;
    }
    if let Some(key) = project_vault_get_api_key(provider) {
        eprintln!("hydra-llm: API key loaded from project vault/{provider}.toml");
        return key;
    }
    eprintln!("hydra-llm: no API key found for {provider} (checked {env_key}, vault)");
    String::new()
}

fn vault_get_api_key(provider: &str) -> Option<String> {
    let vault_dir = dirs::home_dir()?.join(".hydra/vault");
    read_vault_toml(&vault_dir.join(format!("{provider}.toml")))
}

fn project_vault_get_api_key(provider: &str) -> Option<String> {
    let vault_path = std::path::Path::new("vault").join(format!("{provider}.toml"));
    read_vault_toml(&vault_path)
}

/// Load .env file into process environment. Walks up from cwd.
pub fn load_dotenv() {
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let env_path = d.join(".env");
        if env_path.is_file() {
            if let Ok(contents) = std::fs::read_to_string(&env_path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') { continue; }
                    if let Some((key, val)) = line.split_once('=') {
                        let key = key.trim();
                        let val = val.trim();
                        if std::env::var(key).is_err() {
                            unsafe { std::env::set_var(key, val) };
                        }
                    }
                }
            }
            return;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
}

fn read_vault_toml(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let table: toml::Table = content.parse().ok()?;
    let creds = table.get("credentials")?.as_table()?;
    creds.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string())
        .or_else(|| creds.get("token").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .filter(|s| !s.is_empty() && !s.contains("your-"))
}
