use std::io::{self, Write};
use std::path::PathBuf;

use hydra_db::HydraDb;
use hydra_server::{start_server, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging — quiet by default, verbose with RUST_LOG=debug
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
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

    // ── Progressive startup display ──
    let version = env!("CARGO_PKG_VERSION");
    println!();
    println!("  \x1b[34m◉\x1b[0m \x1b[1mHydra Server v{}\x1b[0m", version);
    println!();

    progress(0, "Initializing filesystem...");
    init_filesystem(&data_dir);
    progress(20, "Filesystem ready");

    progress(30, "Initializing database...");
    let db_path = data_dir.join("hydra.db");
    let db = HydraDb::init(&db_path).expect("Failed to initialize database");
    progress(50, "Running migrations...");
    db.migrate().expect("Failed to run migrations");
    progress(60, "Database ready");

    progress(70, "Creating server state...");
    let has_auth = auth_token.is_some();
    let state = AppState::new(db, server_mode, auth_token);
    progress(85, "Server state initialized");

    progress(95, &format!("Binding to port {}...", port));
    // Clear the progress line for final message
    print!("\r\x1b[K");
    println!("  \x1b[32m✓\x1b[0m Filesystem   \x1b[2m{}\x1b[0m", data_dir.display());
    println!("  \x1b[32m✓\x1b[0m Database     \x1b[2m{}\x1b[0m", db_path.display());
    println!("  \x1b[32m✓\x1b[0m Server mode  \x1b[2m{}\x1b[0m", if server_mode { "enabled" } else { "local" });
    println!("  \x1b[32m✓\x1b[0m Auth         \x1b[2m{}\x1b[0m", if has_auth { "token set" } else { "none (local only)" });
    println!();
    println!("  \x1b[32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m 100%");
    println!();
    println!("  \x1b[1m\x1b[32m⚡ Hydra server listening on http://0.0.0.0:{}\x1b[0m", port);
    println!("  \x1b[2mPress Ctrl+C to stop\x1b[0m");
    println!();

    if let Err(e) = start_server(state, port).await {
        eprintln!("  \x1b[31m✗\x1b[0m Server error: {e}");
        std::process::exit(1);
    }
}

fn progress(pct: u8, msg: &str) {
    let bar_width = 30;
    let filled = (pct as usize * bar_width) / 100;
    let empty = bar_width - filled;
    print!(
        "\r  \x1b[34m{}{}\x1b[0m {:3}%  \x1b[2m{}\x1b[0m\x1b[K",
        "━".repeat(filled),
        "╌".repeat(empty),
        pct,
        msg
    );
    io::stdout().flush().unwrap_or_default();
    if pct < 100 {
        std::thread::sleep(std::time::Duration::from_millis(50));
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
