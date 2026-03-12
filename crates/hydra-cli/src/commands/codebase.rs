//! Codebase commands — interact with the Codebase sister

use crate::output;

pub fn analyze(path: Option<&str>) {
    let path = path.unwrap_or(".");
    output::print_header("Codebase Analysis");
    output::print_info(&format!("Analyzing codebase at: {}", path));
    output::print_kv("Status", "Codebase sister not connected (offline mode)");
}

pub fn search(query: &str) {
    output::print_header("Codebase Search");
    output::print_info(&format!("Searching codebase for: {}", query));
    output::print_kv("Status", "Codebase sister not connected (offline mode)");
}

pub fn impact(target: &str) {
    output::print_header("Impact Analysis");
    output::print_info(&format!("Analyzing impact of changes to: {}", target));
    output::print_kv("Status", "Codebase sister not connected (offline mode)");
}

pub fn stats() {
    output::print_header("Codebase Stats");
    output::print_kv("Files tracked", "0");
    output::print_kv("Concepts indexed", "0");
    output::print_kv("Status", "Codebase sister not connected (offline mode)");
}
