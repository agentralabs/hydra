//! Slash command handler for /improve-sister (P10).

use super::app::{App, Message, MessageRole};

impl App {
    /// Handle /improve-sister <path> <goal>
    pub(crate) fn slash_cmd_improve_sister(&mut self, args: &str, timestamp: &str) {
        if args.trim().is_empty() {
            push_si(self, timestamp, concat!(
                "Usage: /improve-sister <path> <goal>\n\n",
                "Examples:\n",
                "  /improve-sister ../agentic-memory add retry logic to MCP transport\n",
                "  /improve-sister ../agentic-codebase --auto\n\n",
                "Hydra will analyze the sister, run baseline tests, generate an improvement ",
                "patch, apply it with a safety checkpoint, re-run tests, and auto-revert ",
                "if anything regresses."
            ));
            return;
        }

        let full_text = format!("/improve-sister {}", args);

        let sister_path = match hydra_native::sister_improve::extract_sister_path(&full_text) {
            Some(p) => p,
            None => {
                push_si(self, timestamp,
                    "Could not find a valid path. Provide an existing directory path.");
                return;
            }
        };

        if !sister_path.exists() {
            push_si(self, timestamp,
                &format!("Path does not exist: {}", sister_path.display()));
            return;
        }

        let goal = hydra_native::sister_improve::extract_goal(&full_text);

        push_si(self, timestamp,
            &format!("Starting sister improvement on {}\nGoal: {}",
                sister_path.display(), goal));

        // Run improvement pipeline in background
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let path = sister_path.clone();
        let goal_clone = goal.clone();

        tokio::spawn(async move {
            let improver = hydra_native::sister_improve::SisterImprover::new();
            let report = improver.improve(&path, &goal_clone, &tx).await;
            eprintln!("[sister-improve] Result: {}", report.summary());
        });

        // Drain any immediate updates
        while let Ok(update) = rx.try_recv() {
            if let hydra_native::CognitiveUpdate::Phase(msg) = update
            {
                push_si(self, timestamp, &msg);
            }
        }
    }
}

fn push_si(app: &mut App, timestamp: &str, content: &str) {
    app.messages.push(Message {
        role: MessageRole::System,
        content: content.to_string(),
        timestamp: timestamp.to_string(),
        phase: Some("sister-improve".into()),
    });
}
