use std::path::PathBuf;

use hydra_db::HydraDb;
use hydra_server::{start_server, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Determine config from env
    let port: u16 = std::env::var("HYDRA_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7777);

    let data_dir = std::env::var("HYDRA_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_home().join(".hydra"));

    let server_mode = std::env::var("HYDRA_SERVER_MODE").is_ok();
    let auth_token = std::env::var("AGENTIC_TOKEN").ok();

    // Initialize filesystem
    init_filesystem(&data_dir);

    // Initialize database
    let db_path = data_dir.join("hydra.db");
    let db = HydraDb::init(&db_path).expect("Failed to initialize database");
    db.migrate().expect("Failed to run migrations");

    tracing::info!("Database initialized at {}", db_path.display());

    // Create server state
    let state = AppState::new(db, server_mode, auth_token);

    // Start server (heartbeat is spawned inside start_server)
    tracing::info!("Starting Hydra server on port {port}");
    if let Err(e) = start_server(state, port).await {
        tracing::error!("Server error: {e}");
        std::process::exit(1);
    }
}

fn init_filesystem(data_dir: &PathBuf) {
    let subdirs = ["receipts", "evidence", "cache", "logs", "voice"];
    for dir in &subdirs {
        let path = data_dir.join(dir);
        if let Err(e) = std::fs::create_dir_all(&path) {
            tracing::warn!("Failed to create {}: {e}", path.display());
        }
    }

    // Set directory permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700));
        for dir in &subdirs {
            let path = data_dir.join(dir);
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700));
        }
    }

    tracing::info!("Filesystem initialized at {}", data_dir.display());
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
