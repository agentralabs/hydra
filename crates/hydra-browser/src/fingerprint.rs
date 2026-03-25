//! Browser fingerprint stealth — defeats headless detection.
//! Applied during BrowserEngine::launch() to make Chrome appear as a real user browser.

/// Browser identity profile for stealth mode.
#[derive(Debug, Clone)]
pub struct BrowserProfile {
    pub user_agent: String,
    pub platform: String,
    pub locale: String,
}

/// Returns a realistic browser profile (Chrome on macOS).
pub fn default_profile() -> BrowserProfile {
    // Randomize minor version for fingerprint variance
    let minor = 100 + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 20);
    BrowserProfile {
        user_agent: format!("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{minor}.0.0.0 Safari/537.36"),
        platform: "MacIntel".into(),
        locale: "en-US".into(),
    }
}

/// Chrome CLI args that defeat headless/automation detection (EC-12.1).
pub fn stealth_args() -> Vec<String> {
    vec![
        "--disable-blink-features=AutomationControlled".into(),
        "--disable-infobars".into(),
        "--no-first-run".into(),
        "--disable-default-apps".into(),
        "--disable-background-networking".into(),
        "--disable-sync".into(),
        "--disable-translate".into(),
        "--metrics-recording-only".into(),
    ]
}

/// JavaScript to inject on every new page to complete stealth (EC-12.1).
/// Sets navigator.webdriver=false, patches plugin list, removes automation indicators.
pub fn stealth_js() -> &'static str {
    r#"
    Object.defineProperty(navigator, 'webdriver', { get: () => false });
    Object.defineProperty(navigator, 'plugins', {
        get: () => [
            { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer' },
            { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai' },
            { name: 'Native Client', filename: 'internal-nacl-plugin' },
        ]
    });
    Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
    Object.defineProperty(navigator, 'deviceMemory', { get: () => 8 });
    Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => 4 });
    Object.defineProperty(screen, 'width', { get: () => 1920 });
    Object.defineProperty(screen, 'height', { get: () => 1080 });
    Object.defineProperty(window, 'outerWidth', { get: () => 1920 });
    Object.defineProperty(window, 'outerHeight', { get: () => 1040 });
    if (window.chrome) { window.chrome.runtime = { connect: () => {}, sendMessage: () => {} }; }
    "#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stealth_args_contain_automation_flag() {
        let args = stealth_args();
        assert!(args.iter().any(|a| a.contains("AutomationControlled")));
        assert!(args.iter().any(|a| a.contains("--no-first-run")));
    }

    #[test]
    fn stealth_js_patches_webdriver() {
        let js = stealth_js();
        assert!(js.contains("webdriver"));
        assert!(js.contains("plugins"));
        assert!(js.contains("languages"));
    }

    #[test]
    fn default_profile_is_realistic() {
        let profile = default_profile();
        assert!(profile.user_agent.contains("Chrome/"));
        assert!(profile.user_agent.contains("Macintosh"));
        assert_eq!(profile.platform, "MacIntel");
    }
}
