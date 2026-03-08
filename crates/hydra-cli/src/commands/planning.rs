//! Planning commands — interact with the Planning sister

use crate::output;

pub fn create(description: &str) {
    output::print_header("Create Plan");
    output::print_info(&format!("Creating plan for: {}", description));
    output::print_kv("Status", "Planning sister not connected (offline mode)");
}

pub fn list() {
    output::print_header("Active Plans");
    output::print_info("No active plans");
    output::print_kv("Status", "Planning sister not connected (offline mode)");
}

pub fn show(plan_id: &str) {
    output::print_header("Plan Details");
    output::print_info(&format!("Showing plan: {}", plan_id));
    output::print_kv("Status", "Planning sister not connected (offline mode)");
}

pub fn progress(plan_id: &str) {
    output::print_header("Plan Progress");
    output::print_info(&format!("Progress for plan: {}", plan_id));
    output::print_kv("Status", "Planning sister not connected (offline mode)");
}
