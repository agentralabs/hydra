// About page — data directories, per-sister export, backup
// Included inside app_rsx_settings_about.rs
{
    let sisters: Vec<(&str, &str, &str)> = vec![
        ("Memory",   ".amem",    "Conversations, facts, knowledge"),
        ("Vision",   ".avis",    "Screenshots, visual context"),
        ("Codebase", ".acb",     "Code understanding graphs"),
        ("Identity", ".aid",     "Trust scores, verification"),
        ("Time",     ".atime",   "Temporal patterns, deadlines"),
        ("Contract", ".acon",    "Policies, approvals, obligations"),
        ("Comm",     ".acomm",   "Message history"),
        ("Planning", ".aplan",   "Goals, progress, plans"),
        ("Cognition",".acog",    "User model, decision patterns"),
        ("Reality",  ".areal",   "Environment snapshots"),
        ("Veritas",  ".averitas","Truth verification records"),
        ("Aegis",    ".aegis",   "Security audit trail"),
        ("Evolve",   ".aevolve", "Learned patterns, skills"),
        ("Forge",    ".aforge",  "Architecture blueprints"),
    ];
    let hydra_dirs: Vec<(&str, &str, &str)> = vec![
        ("Database", ".hydra/hydra.db",    "Conversations, sessions"),
        ("Profile",  ".hydra/profile.json","Settings, API keys"),
        ("Sessions", ".hydra/sessions/",   "Session transcripts"),
        ("Receipts", ".hydra/receipts/",   "Execution audit trail"),
        ("Beliefs",  ".hydra/beliefs/",    "Learned corrections"),
    ];
    rsx! {
        // ── Portability story ──
        p { class: "about-portability-note",
            "Each sister is a standalone open-source package. Your data works without Hydra \u{2014} export it, move it, use it with any MCP client."
        }
        // ── Sister Data ──
        h4 { class: "about-data-heading", "Sister Data" }
        div { class: "about-data-table",
            for (name, dir, desc) in sisters.iter() {
                { let dir_path = format!("{}/.{}", crate::platform::home_dir(),
                    dir.trim_start_matches('.'));
                  let exists = std::path::Path::new(&dir_path).exists();
                  let dir_s = dir.to_string();
                  let name_s = name.to_string();
                  rsx! {
                    div { class: "about-data-row",
                        div { class: "about-data-info",
                            span { class: "about-data-name", "{name}" }
                            span { class: "about-data-path", "~/{dir_s}/" }
                            span { class: "about-data-desc", "{desc}" }
                        }
                        div { class: "about-data-actions",
                            if exists {
                                button {
                                    class: "btn-mini",
                                    title: "Export as zip to Downloads",
                                    onclick: {
                                        let dir_c = dir_s.clone();
                                        let name_c = name_s.clone();
                                        move |_| {
                                            let d = dir_c.clone();
                                            let n = name_c.clone();
                                            let mut status = backup_status.clone();
                                            spawn(async move {
                                                let home = crate::platform::home_dir();
                                                let src = format!("{}/{}", home, d);
                                                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                                                let zip = format!("{}/Downloads/{}-export-{}.zip", home, n.to_lowercase(), date);
                                                match crate::platform::zip_directory(&src, &zip) {
                                                    Ok(()) => status.set(format!("done: {}", zip)),
                                                    Err(e) => status.set(format!("error: {}", e)),
                                                }
                                            });
                                        }
                                    },
                                    "Export"
                                }
                                button {
                                    class: "btn-mini btn-mini-secondary",
                                    title: "Open in Finder",
                                    onclick: {
                                        let dir_c = dir_s.clone();
                                        move |_| {
                                            crate::platform::open_path(&format!("{}/{}", crate::platform::home_dir(), dir_c));
                                        }
                                    },
                                    "Open"
                                }
                            } else {
                                span { class: "about-data-empty", "No data yet" }
                            }
                        }
                    }
                } }
            }
        }
        // ── Hydra Config ──
        h4 { class: "about-data-heading", "Hydra Config" }
        div { class: "about-data-table",
            for (name, path, desc) in hydra_dirs.iter() {
                div { class: "about-data-row",
                    div { class: "about-data-info",
                        span { class: "about-data-name", "{name}" }
                        span { class: "about-data-path", "~/{path}" }
                        span { class: "about-data-desc", "{desc}" }
                    }
                }
            }
        }
        // ── Backup Actions ──
        div { class: "about-backup-section",
            div { class: "about-backup-actions",
                button {
                    class: if is_backing_up { "btn-primary backup-btn disabled" } else { "btn-primary backup-btn" },
                    disabled: is_backing_up,
                    onclick: move |_| {
                        backup_status.set("running".to_string());
                        let mut status = backup_status.clone();
                        spawn(async move {
                            let home = crate::platform::home_dir();
                            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                            let zip_path = format!("{}/Downloads/hydra-backup-{}.zip", home, date);
                            let dirs: Vec<String> = [
                                ".hydra", ".amem", ".avis", ".acb", ".aid", ".atime",
                                ".acon", ".acomm", ".aplan", ".acog", ".areal",
                                ".averitas", ".aegis", ".aevolve", ".aforge",
                            ].iter()
                                .map(|d| format!("{}/{}", home, d))
                                .filter(|p| std::path::Path::new(p).exists())
                                .collect();
                            if dirs.is_empty() {
                                status.set("error: No data directories found".to_string());
                                return;
                            }
                            // Zip all existing dirs into one backup
                            let args: Vec<&str> = std::iter::once("-r").chain(std::iter::once("-q"))
                                .chain(std::iter::once(zip_path.as_str()))
                                .chain(dirs.iter().map(|s| s.as_str())).collect();
                            match std::process::Command::new("zip").args(&args).output() {
                                Ok(o) if o.status.success() => status.set(format!("done: {}", zip_path)),
                                Ok(o) => status.set(format!("error: {}", String::from_utf8_lossy(&o.stderr).trim())),
                                Err(e) => status.set(format!("error: zip not available: {}", e)),
                            }
                        });
                    },
                    if is_backing_up { "Backing up..." } else { "Back Up Everything" }
                }
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        crate::platform::open_path(&format!("{}/.hydra", crate::platform::home_dir()));
                    },
                    "Open ~/.hydra/ in Finder"
                }
            }
            p { class: "about-backup-hint",
                "Exports all sister data + Hydra config into one zip in ~/Downloads/. Move to any machine to restore."
            }
        }
        // ── Status feedback ──
        if backup_text.starts_with("done:") {
            { let path = backup_text.strip_prefix("done: ").unwrap_or("");
              rsx! {
                div { class: "about-backup-success",
                    span { "\u{2705} Saved to " }
                    strong { "{path}" }
                    button {
                        class: "btn-link",
                        onclick: move |_| {
                            let p = backup_status.read().clone();
                            if let Some(path) = p.strip_prefix("done: ") {
                                crate::platform::reveal_in_finder(path);
                            }
                        },
                        "Show in Finder"
                    }
                }
            } }
        }
        if backup_text.starts_with("error:") {
            { let msg = backup_text.strip_prefix("error: ").unwrap_or("");
              rsx! { p { class: "about-backup-error", "Backup failed: {msg}" } } }
        }
    }
}
