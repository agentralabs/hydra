use crate::commands;
use crate::commands::config::ConfigAction;
use crate::output;

pub fn dispatch_config(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("show");
    match sub {
        "show" => commands::config::execute(ConfigAction::Show),
        "set" => {
            if let (Some(key), Some(val)) = (args.get(3), args.get(4)) {
                commands::config::execute(ConfigAction::Set(
                    key.clone(),
                    val.clone(),
                ));
            } else {
                output::print_error("Usage: hydra config set <key> <value>");
            }
        }
        "get" => {
            if let Some(key) = args.get(3) {
                commands::config::execute(ConfigAction::Get(key.clone()));
            } else {
                output::print_error("Usage: hydra config get <key>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown config subcommand: {}", sub));
            output::print_info("Subcommands: show, set, get");
        }
    }
}

pub fn dispatch_sisters(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
    match sub {
        "status" | "" => commands::sisters::status(),
        "connect" => {
            if let Some(name) = args.get(3) {
                commands::sisters::connect(name);
            } else {
                output::print_error("Usage: hydra sisters connect <name>");
            }
        }
        "disconnect" => {
            if let Some(name) = args.get(3) {
                commands::sisters::disconnect(name);
            } else {
                output::print_error("Usage: hydra sisters disconnect <name>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown sisters subcommand: {}", sub));
            output::print_info("Subcommands: status, connect, disconnect");
        }
    }
}

pub fn dispatch_skills(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" | "" => commands::skills::list(),
        "install" => {
            if let Some(name) = args.get(3) {
                commands::skills::install(name);
            } else {
                output::print_error("Usage: hydra skills install <name>");
            }
        }
        "remove" => {
            if let Some(name) = args.get(3) {
                commands::skills::remove(name);
            } else {
                output::print_error("Usage: hydra skills remove <name>");
            }
        }
        "search" => {
            if let Some(query) = args.get(3) {
                commands::skills::search(query);
            } else {
                output::print_error("Usage: hydra skills search <query>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown skills subcommand: {}", sub));
            output::print_info("Subcommands: list, install, remove, search");
        }
    }
}

pub fn dispatch_memory(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
    match sub {
        "query" => {
            if let Some(q) = args.get(3) {
                commands::memory::query(q);
            } else {
                output::print_error("Usage: hydra memory query <query>");
            }
        }
        "add" => {
            if let Some(content) = args.get(3) {
                commands::memory::add(content);
            } else {
                output::print_error("Usage: hydra memory add <content>");
            }
        }
        "stats" | "" => commands::memory::stats(),
        "clear" => commands::memory::clear(args.get(3).map(|s| s.as_str())),
        _ => {
            output::print_error(&format!("Unknown memory subcommand: {}", sub));
            output::print_info("Subcommands: query, add, stats, clear");
        }
    }
}

pub fn dispatch_codebase(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
    match sub {
        "analyze" => commands::codebase::analyze(args.get(3).map(|s| s.as_str())),
        "search" => {
            if let Some(q) = args.get(3) {
                commands::codebase::search(q);
            } else {
                output::print_error("Usage: hydra codebase search <query>");
            }
        }
        "impact" => {
            if let Some(target) = args.get(3) {
                commands::codebase::impact(target);
            } else {
                output::print_error("Usage: hydra codebase impact <target>");
            }
        }
        "stats" | "" => commands::codebase::stats(),
        _ => {
            output::print_error(&format!("Unknown codebase subcommand: {}", sub));
            output::print_info("Subcommands: analyze, search, impact, stats");
        }
    }
}

pub fn dispatch_vision(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("stats");
    match sub {
        "capture" => commands::vision::capture(args.get(3).map(|s| s.as_str())),
        "compare" => {
            if let (Some(a), Some(b)) = (args.get(3), args.get(4)) {
                commands::vision::compare(a, b);
            } else {
                output::print_error("Usage: hydra vision compare <image_a> <image_b>");
            }
        }
        "ocr" => {
            if let Some(path) = args.get(3) {
                commands::vision::ocr(path);
            } else {
                output::print_error("Usage: hydra vision ocr <image_path>");
            }
        }
        "stats" | "" => commands::vision::stats(),
        _ => {
            output::print_error(&format!("Unknown vision subcommand: {}", sub));
            output::print_info("Subcommands: capture, compare, ocr, stats");
        }
    }
}

pub fn dispatch_planning(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "create" => {
            if let Some(desc) = args.get(3) {
                commands::planning::create(desc);
            } else {
                output::print_error("Usage: hydra planning create <description>");
            }
        }
        "list" | "" => commands::planning::list(),
        "show" => {
            if let Some(id) = args.get(3) {
                commands::planning::show(id);
            } else {
                output::print_error("Usage: hydra planning show <plan_id>");
            }
        }
        "progress" => {
            if let Some(id) = args.get(3) {
                commands::planning::progress(id);
            } else {
                output::print_error("Usage: hydra planning progress <plan_id>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown planning subcommand: {}", sub));
            output::print_info("Subcommands: create, list, show, progress");
        }
    }
}

pub fn dispatch_soul(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
    match sub {
        "save" => commands::soul::save(args.get(3).map(|s| s.as_str())),
        "status" | "" => commands::soul::status(),
        "export" => {
            if let Some(path) = args.get(3) {
                commands::soul::export(path);
            } else {
                output::print_error("Usage: hydra soul export <path>");
            }
        }
        "import" => {
            if let Some(path) = args.get(3) {
                commands::soul::import(path);
            } else {
                output::print_error("Usage: hydra soul import <path>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown soul subcommand: {}", sub));
            output::print_info("Subcommands: save, status, export, import");
        }
    }
}

pub fn dispatch_remote(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" | "" => commands::remote::list(),
        "connect" => {
            if let Some(addr) = args.get(3) {
                commands::remote::connect(addr);
            } else {
                output::print_error("Usage: hydra remote connect <address>");
            }
        }
        "disconnect" => {
            if let Some(id) = args.get(3) {
                commands::remote::disconnect(id);
            } else {
                output::print_error("Usage: hydra remote disconnect <instance_id>");
            }
        }
        "sync" => commands::remote::sync(),
        _ => {
            output::print_error(&format!("Unknown remote subcommand: {}", sub));
            output::print_info("Subcommands: list, connect, disconnect, sync");
        }
    }
}

pub fn dispatch_voice(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
    match sub {
        "start" => commands::voice::start(),
        "stop" => commands::voice::stop(),
        "status" | "" => commands::voice::status(),
        _ => {
            output::print_error(&format!("Unknown voice subcommand: {}", sub));
            output::print_info("Subcommands: start, stop, status");
        }
    }
}

pub fn dispatch_policy(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" | "" => commands::policy::list(),
        "add" => {
            if let (Some(name), Some(rule)) = (args.get(3), args.get(4)) {
                commands::policy::add(name, rule);
            } else {
                output::print_error("Usage: hydra policy add <name> <rule>");
            }
        }
        "remove" => {
            if let Some(name) = args.get(3) {
                commands::policy::remove(name);
            } else {
                output::print_error("Usage: hydra policy remove <name>");
            }
        }
        "check" => {
            if let Some(action) = args.get(3) {
                commands::policy::check(action);
            } else {
                output::print_error("Usage: hydra policy check <action>");
            }
        }
        _ => {
            output::print_error(&format!("Unknown policy subcommand: {}", sub));
            output::print_info("Subcommands: list, add, remove, check");
        }
    }
}

pub fn dispatch_serve(args: &[String]) {
    let mut port: u16 = 3000;
    let mut host = "127.0.0.1";
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                i += 1;
                if i < args.len() {
                    port = args[i].parse().unwrap_or(3000);
                }
            }
            "--host" => {
                i += 1;
                if i < args.len() {
                    host = args[i].as_str();
                }
            }
            _ => {}
        }
        i += 1;
    }
    commands::serve::execute(port, host);
}

pub fn dispatch_profile(args: &[String]) {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("show");
    match sub {
        "show" | "" => commands::profile::show(),
        "set-name" => {
            if let Some(name) = args.get(3) {
                commands::profile::set_name(name);
            } else {
                output::print_error("Usage: hydra profile set-name <name>");
            }
        }
        "reset" => commands::profile::reset(),
        "list" => {
            let client = crate::client::HydraClient::new();
            match client.get("/api/profile/list") {
                Ok(data) => {
                    output::print_header("Operational Profiles");
                    if let Some(profiles) = data.as_array() {
                        for p in profiles {
                            let name = p["name"].as_str().unwrap_or("?");
                            let beliefs = p["beliefs"].as_u64().unwrap_or(0);
                            let skills = p["skills"].as_u64().unwrap_or(0);
                            println!("  {} ({} beliefs, {} skills)", name, beliefs, skills);
                        }
                    }
                }
                Err(_) => {
                    output::print_warning("Server offline. Listing from disk.");
                    use hydra_native::operational_profile::list_profiles;
                    match list_profiles() {
                        Ok(names) => { for n in &names { println!("  {}", n); } }
                        Err(e) => output::print_error(&format!("Failed: {}", e)),
                    }
                }
            }
        }
        "load" => {
            if let Some(name) = args.get(3) {
                let client = crate::client::HydraClient::new();
                match client.post("/api/profile/load", &serde_json::json!({"name": name})) {
                    Ok(_) => output::print_success(&format!("Profile '{}' loaded", name)),
                    Err(_) => {
                        output::print_warning("Server offline. Loading locally.");
                        match hydra_native::cognitive::profile_loader::load_profile(name) {
                            Ok(p) => {
                                let b = p.beliefs.len();
                                let s = p.skills.len();
                                output::print_success(&format!(
                                    "Loaded '{}' ({} beliefs, {} skills)", name, b, s
                                ));
                            }
                            Err(e) => output::print_error(&format!("Failed: {}", e)),
                        }
                    }
                }
            } else {
                output::print_error("Usage: hydra profile load <name>");
            }
        }
        "info" => {
            let name = args.get(3).map(|s| s.as_str()).unwrap_or("dev");
            match hydra_native::cognitive::profile_loader::load_profile(name) {
                Ok(p) => {
                    output::print_header(&format!("Profile: {}", name));
                    if let Some(id) = &p.identity {
                        println!("  Persona: {}", id.persona);
                    }
                    println!("  Beliefs: {}", p.beliefs.len());
                    println!("  Skills:  {}", p.skills.len());
                    println!("  Goals:   {}", p.goals.len());
                    if let Some(m) = &p.model {
                        println!("  Model:   {}", m.default);
                    }
                }
                Err(e) => output::print_error(&format!("Failed: {}", e)),
            }
        }
        "beliefs" => {
            let client = crate::client::HydraClient::new();
            match client.get("/api/profile/beliefs") {
                Ok(data) => {
                    output::print_header("Active Beliefs");
                    if let Some(beliefs) = data.as_array() {
                        for b in beliefs {
                            let fact = b["fact"].as_str().unwrap_or("?");
                            let conf = b["confidence"].as_f64().unwrap_or(0.0);
                            println!("  [{:.0}%] {}", conf * 100.0, fact);
                        }
                        println!("\n  Total: {} beliefs", beliefs.len());
                    }
                }
                Err(_) => output::print_warning("Server offline. Load a profile first."),
            }
        }
        "skills" => {
            let client = crate::client::HydraClient::new();
            match client.get("/api/profile/skills") {
                Ok(data) => {
                    output::print_header("Active Skills");
                    if let Some(skills) = data.as_array() {
                        for s in skills {
                            let name = s["name"].as_str().unwrap_or("?");
                            let desc = s["description"].as_str().unwrap_or("");
                            println!("  {} — {}", name, desc);
                        }
                        println!("\n  Total: {} skills", skills.len());
                    }
                }
                Err(_) => output::print_warning("Server offline. Load a profile first."),
            }
        }
        "unload" => {
            let client = crate::client::HydraClient::new();
            match client.post("/api/profile/unload", &serde_json::json!({})) {
                Ok(_) => output::print_success("Profile unloaded"),
                Err(_) => output::print_warning("Server offline."),
            }
        }
        _ => {
            output::print_error(&format!("Unknown profile subcommand: {}", sub));
            output::print_info("Subcommands: show, list, load, info, beliefs, skills, unload, set-name, reset");
        }
    }
}

pub fn dispatch_logs(args: &[String]) {
    let mut follow = false;
    let mut level: Option<&str> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--follow" | "-f" => follow = true,
            "--level" => {
                i += 1;
                if i < args.len() {
                    level = Some(args[i].as_str());
                }
            }
            _ => {}
        }
        i += 1;
    }
    commands::logs::execute(follow, level);
}
