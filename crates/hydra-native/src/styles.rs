//! CSS styles for the native desktop app.
//!
//! Embedded as a const string — injected into the webview at startup.

/// Premium dark theme CSS (matches hydra-desktop Next.js styles)
pub const STYLES: &str = r#"
:root {
    --bg-primary: #0a0a0f;
    --bg-secondary: #12121a;
    --bg-glass: rgba(255, 255, 255, 0.05);
    --text-primary: #e2e8f0;
    --text-secondary: #94a3b8;
    --accent: #6366f1;
    --accent-hover: #818cf8;
    --success: #22c55e;
    --error: #ef4444;
    --warning: #f59e0b;
    --border: rgba(255, 255, 255, 0.08);
    --radius: 12px;
    --font: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
    font-family: var(--font);
    background: var(--bg-primary);
    color: var(--text-primary);
    overflow: hidden;
    height: 100vh;
}

/* Globe animations */
@keyframes globe-breathe {
    0%, 100% { transform: scale(1); opacity: 0.9; }
    50% { transform: scale(1.05); opacity: 1; }
}

@keyframes globe-pulse {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.1); }
}

@keyframes globe-rotate {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
}

@keyframes globe-shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-3px); }
    75% { transform: translateX(3px); }
}

@keyframes globe-glow {
    0%, 100% { box-shadow: 0 0 20px rgba(245, 158, 11, 0.3); }
    50% { box-shadow: 0 0 40px rgba(245, 158, 11, 0.6); }
}

@keyframes globe-ring-out {
    0% { transform: scale(1); opacity: 0.6; }
    100% { transform: scale(1.5); opacity: 0; }
}

.globe-idle { animation: globe-breathe 3s ease-in-out infinite; }
.globe-listening { animation: globe-pulse 1.5s ease-in-out infinite; }
.globe-processing { animation: globe-rotate 2s linear infinite; }
.globe-speaking { animation: globe-ring-out 1s ease-out infinite; }
.globe-error { animation: globe-shake 0.5s ease-in-out; }
.globe-approval { animation: globe-glow 2s ease-in-out infinite; }

/* Phase indicator */
.phase-pending { background: var(--text-secondary); opacity: 0.4; }
.phase-running { background: var(--accent); animation: globe-pulse 1s ease-in-out infinite; }
.phase-completed { background: var(--success); }
.phase-failed { background: var(--error); }

.connector-active { background: var(--success); height: 2px; }
.connector-inactive { background: var(--border); height: 2px; }

/* Messages */
.message-user {
    background: linear-gradient(135deg, var(--accent), #4f46e5);
    border-radius: var(--radius);
    padding: 12px 16px;
    margin: 4px 0;
    max-width: 80%;
    align-self: flex-end;
}

.message-hydra {
    background: var(--bg-glass);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 16px;
    margin: 4px 0;
    max-width: 80%;
    backdrop-filter: blur(10px);
}

/* Input */
.chat-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    padding: 12px 16px;
    width: 100%;
    font-size: 14px;
    outline: none;
    transition: border-color 0.2s;
}
.chat-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

/* Code blocks */
pre {
    background: #1e1e2e;
    border-radius: 8px;
    padding: 12px;
    overflow-x: auto;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
}

code {
    background: rgba(99, 102, 241, 0.15);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
}

/* Scrollbar */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: var(--text-secondary); }

/* Fade in animation */
@keyframes fade-in {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
}
.fade-in { animation: fade-in 0.3s ease-out; }
"#;

/// Get the full HTML wrapper for the Dioxus app
pub fn html_wrapper(body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Hydra</title>
    <style>{css}</style>
</head>
<body>{body}</body>
</html>"#,
        css = STYLES,
        body = body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styles_contains_variables() {
        assert!(STYLES.contains("--bg-primary"));
        assert!(STYLES.contains("--accent"));
        assert!(STYLES.contains("globe-breathe"));
    }

    #[test]
    fn test_styles_contains_all_globe_classes() {
        assert!(STYLES.contains(".globe-idle"));
        assert!(STYLES.contains(".globe-listening"));
        assert!(STYLES.contains(".globe-processing"));
        assert!(STYLES.contains(".globe-speaking"));
        assert!(STYLES.contains(".globe-error"));
        assert!(STYLES.contains(".globe-approval"));
    }

    #[test]
    fn test_styles_contains_phase_classes() {
        assert!(STYLES.contains(".phase-pending"));
        assert!(STYLES.contains(".phase-running"));
        assert!(STYLES.contains(".phase-completed"));
        assert!(STYLES.contains(".phase-failed"));
    }

    #[test]
    fn test_html_wrapper() {
        let html = html_wrapper("<div>Hello</div>");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<div>Hello</div>"));
        assert!(html.contains("--bg-primary"));
    }
}
