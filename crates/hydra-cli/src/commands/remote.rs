//! Remote commands — manage distributed Hydra instances

use crate::output;

pub fn list() {
    output::print_header("Remote Instances");
    output::print_info("No remote instances connected");
    output::print_kv("Status", "Distributed mode not yet implemented");
}

pub fn connect(address: &str) {
    output::print_header("Connect Remote");
    output::print_info(&format!("Connecting to: {}", address));
    output::print_kv("Status", "Remote connection not yet implemented");
}

pub fn disconnect(instance_id: &str) {
    output::print_header("Disconnect Remote");
    output::print_info(&format!("Disconnecting: {}", instance_id));
    output::print_kv("Status", "Remote disconnection not yet implemented");
}

pub fn sync() {
    output::print_header("Sync Remote");
    output::print_info("Syncing with remote instances...");
    output::print_kv("Status", "Remote sync not yet implemented");
}
