//! Browser task — async browser automation spawned from conversation.
//! Reuses the same channel pattern as LLM streaming.

use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum BrowserUpdate {
    Status(String),
    Error(String),
    Done { url: String, title: String, text_preview: String },
}

/// Spawn a browser task. Returns a receiver for progress updates.
pub fn spawn(rt: &tokio::runtime::Runtime, goal: String) -> mpsc::Receiver<BrowserUpdate> {
    let (tx, rx) = mpsc::channel(32);
    rt.spawn(async move { run(goal, tx).await });
    rx
}

async fn run(goal: String, tx: mpsc::Sender<BrowserUpdate>) {
    let _ = tx.send(BrowserUpdate::Status(format!("Browsing: {goal}"))).await;
    let url = extract_url(&goal).unwrap_or_else(|| goal.clone());
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{url}")
    } else { url };

    // Wrap entire browser operation in catch_unwind to prevent TUI crashes
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Nothing here — the actual async work is below
    }));
    if result.is_err() {
        let _ = tx.send(BrowserUpdate::Error("Browser panicked — Chrome may not be available".into())).await;
        return;
    }

    // Try Chrome with timeout
    let mut engine = hydra_browser::BrowserEngine::new();
    match tokio::time::timeout(std::time::Duration::from_secs(30), engine.launch()).await {
        Ok(Ok(_)) => {} // launched
        Ok(Err(e)) => {
            let _ = tx.send(BrowserUpdate::Error(format!("Chrome launch failed: {e}"))).await;
            return;
        }
        Err(_) => {
            let _ = tx.send(BrowserUpdate::Error("Chrome launch timed out (30s)".into())).await;
            return;
        }
    }
    let _ = tx.send(BrowserUpdate::Status("Chrome launched, navigating...".into())).await;
    match tokio::time::timeout(std::time::Duration::from_secs(30), engine.navigate(&url)).await {
        Ok(Ok(_)) => {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                engine.execute(&hydra_browser::BrowserAction::GetText)
            ).await;
            match result {
                Ok(r) if r.success => {
                    let text = truncate(&r.data, 2000);
                    let _ = tx.send(BrowserUpdate::Done { url, title: "Page loaded".into(), text_preview: text }).await;
                }
                Ok(r) => { let _ = tx.send(BrowserUpdate::Error(r.error.unwrap_or_default())).await; }
                Err(_) => { let _ = tx.send(BrowserUpdate::Error("Page text extraction timed out".into())).await; }
            }
            engine.close().await;
        }
        Ok(Err(e)) => {
            let _ = tx.send(BrowserUpdate::Error(format!("Navigation failed: {e}"))).await;
            engine.close().await;
        }
        Err(_) => {
            let _ = tx.send(BrowserUpdate::Error("Navigation timed out (30s)".into())).await;
            engine.close().await;
            // Fallback: HTTP fetch
            let _ = tx.send(BrowserUpdate::Status("HTTP fetch (Chrome not available)...".into())).await;
            match reqwest::get(&url).await {
                Ok(resp) => {
                    let status = resp.status();
                    match resp.text().await {
                        Ok(body) => {
                            let text = hydra_browser::PageAnalyzer::extract_text(&body);
                            let _ = tx.send(BrowserUpdate::Done {
                                url, title: format!("HTTP {status}"), text_preview: truncate(&text, 2000),
                            }).await;
                        }
                        Err(e) => { let _ = tx.send(BrowserUpdate::Error(format!("Read: {e}"))).await; }
                    }
                }
                Err(e) => { let _ = tx.send(BrowserUpdate::Error(format!("Fetch: {e}"))).await; }
            }
        }
    }
}

fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            return Some(word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c)).to_string());
        }
        if word.contains('.') && !word.starts_with('.') && word.len() > 3 {
            let clean = word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c));
            if clean.contains('.') { return Some(clean.to_string()); }
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}...", &s[..max]) } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_url_from_text() {
        assert_eq!(extract_url("open https://example.com"), Some("https://example.com".into()));
        assert_eq!(extract_url("go to linkedin.com"), Some("linkedin.com".into()));
    }

    #[test]
    fn extract_url_none() {
        assert_eq!(extract_url("hello world"), None);
    }
}
