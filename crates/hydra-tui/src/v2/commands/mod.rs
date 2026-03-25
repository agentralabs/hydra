//! Command system — single registry, fuzzy searchable.
//! All commands defined in one place with metadata.
//! No more 4-file scatter pattern.

pub mod registry;
pub mod agent;
pub mod core;
pub mod info;
pub mod companion;
pub mod session;
pub mod system;

use registry::{CommandRegistry, Command};

/// Build the full command registry with all commands.
pub fn build_registry() -> CommandRegistry {
    let mut commands: Vec<Command> = Vec::new();
    commands.extend(core::commands());
    commands.extend(session::commands());
    commands.extend(info::commands());
    commands.extend(companion::commands());
    commands.extend(system::commands());
    commands.extend(agent::commands());
    CommandRegistry::new(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_commands() {
        let reg = build_registry();
        // Should have 25+ commands
        assert!(reg.count() >= 20, "Only {} commands registered", reg.count());
    }

    #[test]
    fn no_duplicate_names() {
        let reg = build_registry();
        let mut names: Vec<&str> = reg.all().iter().map(|c| c.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "Duplicate command names found");
    }

    #[test]
    fn help_is_findable() {
        let reg = build_registry();
        assert!(reg.find("help").is_some());
        assert!(reg.find("h").is_some()); // alias
    }

    #[test]
    fn fuzzy_search_works() {
        let reg = build_registry();
        let results = reg.fuzzy_search("bck");
        assert!(!results.is_empty());
    }
}
