//! Memory commands — interact with the Memory sister

use crate::output;

pub fn query(query: &str) {
    output::print_header("Memory Query");
    output::print_info(&format!("Querying memory for: {}", query));
    output::print_kv("Status", "Memory sister not connected (offline mode)");
    output::print_info("Start the memory sister with: hydra sisters connect memory");
}

pub fn add(content: &str) {
    output::print_header("Memory Add");
    output::print_info(&format!("Adding to memory: {}", content));
    output::print_kv("Status", "Memory sister not connected (offline mode)");
}

pub fn stats() {
    output::print_header("Memory Stats");
    output::print_kv("Total memories", "0");
    output::print_kv("Sessions", "0");
    output::print_kv("Cache hit rate", "N/A");
    output::print_kv("Status", "Memory sister not connected (offline mode)");
}

pub fn clear(scope: Option<&str>) {
    let scope = scope.unwrap_or("session");
    output::print_header("Memory Clear");
    output::print_warning(&format!("Clearing {} memories", scope));
    output::print_kv("Status", "Memory sister not connected (offline mode)");
}
