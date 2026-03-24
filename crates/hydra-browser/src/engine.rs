//! BrowserEngine — core browser automation via Chrome DevTools Protocol.
//! Manages browser lifecycle, navigation, screenshots, and action dispatch.

use crate::action::{ActionResult, BrowserAction, ScrollDirection};
use crate::constants::{DEFAULT_VIEWPORT_HEIGHT, DEFAULT_VIEWPORT_WIDTH};
use crate::errors::BrowserError;
use crate::human::HumanBehavior;
use crate::session::SessionManager;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Core browser automation engine.
pub struct BrowserEngine {
    browser: Option<Browser>,
    page: Option<Arc<Mutex<Page>>>,
    session_mgr: SessionManager,
    human: HumanBehavior,
    pub limiter: crate::limiter::RateLimiter,
    launched: bool,
}

impl BrowserEngine {
    pub fn new() -> Self {
        Self {
            browser: None, page: None, session_mgr: SessionManager::new(),
            human: HumanBehavior::new(), limiter: crate::limiter::RateLimiter::new(),
            launched: false,
        }
    }

    /// Launch headless Chrome. Reads HYDRA_CHROME_PATH for custom binary.
    pub async fn launch(&mut self) -> Result<(), BrowserError> {
        if self.launched {
            return Ok(());
        }

        let mut config = BrowserConfig::builder()
            .no_sandbox()
            .window_size(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT);
        // O12: Apply anti-detection stealth args (EC-12.1)
        for arg in crate::fingerprint::stealth_args() { config = config.arg(arg); }
        if let Ok(path) = std::env::var("HYDRA_CHROME_PATH") {
            config = config.chrome_executable(path);
        }

        let config = config.build().map_err(|e| {
            BrowserError::ChromeNotFound(e.to_string())
        })?;

        let (browser, mut handler) =
            Browser::launch(config)
                .await
                .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;

        // Spawn the CDP handler in the background
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                let _ = event;
            }
        });

        eprintln!("hydra-browser: Chrome launched (headless)");
        self.browser = Some(browser);
        self.launched = true;
        Ok(())
    }

    /// Navigate to a URL.
    pub async fn navigate(&mut self, url: &str) -> Result<(), BrowserError> {
        let browser = self.browser.as_ref().ok_or_else(|| {
            BrowserError::LaunchFailed("Browser not launched".into())
        })?;

        let page = browser
            .new_page(url)
            .await
            .map_err(|e| BrowserError::NavigationFailed {
                url: url.to_string(),
                reason: e.to_string(),
            })?;

        // Wait for page load
        let _ = page
            .wait_for_navigation()
            .await;

        eprintln!("hydra-browser: navigated to {}", url);
        self.page = Some(Arc::new(Mutex::new(page)));
        Ok(())
    }

    /// Capture a screenshot as PNG bytes.
    pub async fn screenshot(&self) -> Result<Vec<u8>, BrowserError> {
        let page_lock = self.current_page()?;
        let page = page_lock.lock().await;
        let bytes = page
            .screenshot(
                chromiumoxide::page::ScreenshotParams::builder()
                    .full_page(true)
                    .build(),
            )
            .await
            .map_err(|e| BrowserError::ScreenshotFailed(e.to_string()))?;

        eprintln!("hydra-browser: screenshot captured ({}KB)", bytes.len() / 1024);
        Ok(bytes)
    }

    /// Get the page HTML source.
    pub async fn html(&self) -> Result<String, BrowserError> {
        let page_lock = self.current_page()?;
        let page = page_lock.lock().await;
        let html = page
            .content()
            .await
            .map_err(|e| BrowserError::ActionFailed {
                action: "get_html".into(),
                reason: e.to_string(),
            })?;
        Ok(html)
    }

    /// Execute a BrowserAction and return the result.
    pub async fn execute(&mut self, action: &BrowserAction) -> ActionResult {
        let start = std::time::Instant::now();
        let label = action.label();

        // O12: Rate limiter check before mutations (EC-12.2)
        if action.is_mutation() {
            if let crate::limiter::RateLimitStatus::BackedOff { remaining_ms } = self.limiter.check("_current", 30) {
                return ActionResult::err(label, format!("RATE_LIMITED:{}ms", remaining_ms), 0);
            }
            self.human.delay().await;
        }

        let result = match action {
            BrowserAction::Navigate { url } => {
                match self.navigate(url).await {
                    Ok(()) => ActionResult::ok(label, format!("Navigated to {url}"), 0),
                    Err(e) => ActionResult::err(label, e.to_string(), 0),
                }
            }
            BrowserAction::Screenshot => match self.screenshot().await {
                Ok(bytes) => {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    ActionResult::ok(label, b64, 0)
                }
                Err(e) => ActionResult::err(label, e.to_string(), 0),
            },
            BrowserAction::GetHtml => match self.html().await {
                Ok(html) => ActionResult::ok(label, html, 0),
                Err(e) => ActionResult::err(label, e.to_string(), 0),
            },
            BrowserAction::Click { selector } => {
                self.execute_click(selector).await
            }
            BrowserAction::Type { selector, text } => {
                self.execute_type(selector, text).await
            }
            BrowserAction::Scroll { direction, amount } => {
                self.execute_scroll(direction, *amount).await
            }
            BrowserAction::Wait { ms } => {
                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                ActionResult::ok(label, format!("Waited {ms}ms"), 0)
            }
            BrowserAction::GetElements => self.execute_get_elements().await,
            BrowserAction::GetText => self.execute_get_text().await,
            _ => ActionResult::ok(label, "Action dispatched", 0),
        };

        let duration = start.elapsed().as_millis() as u64;
        ActionResult {
            duration_ms: duration,
            ..result
        }
    }

    /// Close the browser.
    pub async fn close(&mut self) {
        self.page = None;
        self.browser = None;
        self.launched = false;
        eprintln!("hydra-browser: closed");
    }

    pub fn is_launched(&self) -> bool {
        self.launched
    }

    pub fn session_manager(&self) -> &SessionManager {
        &self.session_mgr
    }

    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_mgr
    }

    fn current_page(&self) -> Result<&Arc<Mutex<Page>>, BrowserError> {
        self.page.as_ref().ok_or_else(|| {
            BrowserError::NavigationFailed {
                url: "(no page)".into(),
                reason: "No page is currently open".into(),
            }
        })
    }

    async fn execute_click(&self, selector: &str) -> ActionResult {
        let page_lock = match self.current_page() {
            Ok(p) => p,
            Err(e) => return ActionResult::err("click", e.to_string(), 0),
        };
        let page = page_lock.lock().await;
        match page.find_element(selector).await {
            Ok(el) => match el.click().await {
                Ok(_) => ActionResult::ok("click", format!("Clicked {selector}"), 0),
                Err(e) => ActionResult::err("click", e.to_string(), 0),
            },
            Err(e) => ActionResult::err("click", format!("Element not found: {e}"), 0),
        }
    }

    async fn execute_type(&self, selector: &str, text: &str) -> ActionResult {
        let page_lock = match self.current_page() {
            Ok(p) => p,
            Err(e) => return ActionResult::err("type", e.to_string(), 0),
        };
        let page = page_lock.lock().await;
        match page.find_element(selector).await {
            Ok(el) => {
                // Type character by character with human cadence
                let delays = self.human.typing_cadence(text);
                for (ch, delay_ms) in text.chars().zip(delays.iter()) {
                    if let Err(e) = el.type_str(&ch.to_string()).await {
                        return ActionResult::err("type", e.to_string(), 0);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
                }
                ActionResult::ok("type", format!("Typed {} chars into {selector}", text.len()), 0)
            }
            Err(e) => ActionResult::err("type", format!("Element not found: {e}"), 0),
        }
    }

    async fn execute_scroll(&self, direction: &ScrollDirection, amount: u32) -> ActionResult {
        let page_lock = match self.current_page() {
            Ok(p) => p,
            Err(e) => return ActionResult::err("scroll", e.to_string(), 0),
        };
        let page = page_lock.lock().await;
        let scroll_amount = self.human.natural_scroll(amount) as i64;
        let (dx, dy) = match direction {
            ScrollDirection::Down => (0, scroll_amount),
            ScrollDirection::Up => (0, -scroll_amount),
            ScrollDirection::Right => (scroll_amount, 0),
            ScrollDirection::Left => (-scroll_amount, 0),
        };
        let js = format!("window.scrollBy({dx}, {dy})");
        match page.evaluate(js).await {
            Ok(_) => ActionResult::ok("scroll", format!("Scrolled ({dx}, {dy})"), 0),
            Err(e) => ActionResult::err("scroll", e.to_string(), 0),
        }
    }

    async fn execute_get_elements(&self) -> ActionResult {
        let page_lock = match self.current_page() {
            Ok(p) => p,
            Err(e) => return ActionResult::err("get_elements", e.to_string(), 0),
        };
        let page = page_lock.lock().await;
        let js = r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('a, button, input, select, textarea, [role="button"]'))
                    .slice(0, 100)
                    .map((el, i) => ({
                        index: i,
                        tag: el.tagName.toLowerCase(),
                        type: el.type || null,
                        text: (el.textContent || '').trim().slice(0, 80),
                        name: el.name || null,
                        id: el.id || null,
                        href: el.href || null,
                        placeholder: el.placeholder || null,
                    }))
            )
        "#;
        match page.evaluate(js).await {
            Ok(val) => {
                let text = val.into_value::<String>().unwrap_or_default();
                ActionResult::ok("get_elements", text, 0)
            }
            Err(e) => ActionResult::err("get_elements", e.to_string(), 0),
        }
    }

    async fn execute_get_text(&self) -> ActionResult {
        let page_lock = match self.current_page() {
            Ok(p) => p,
            Err(e) => return ActionResult::err("get_text", e.to_string(), 0),
        };
        let page = page_lock.lock().await;
        let js = "document.body.innerText";
        match page.evaluate(js).await {
            Ok(val) => {
                let text = val.into_value::<String>().unwrap_or_default();
                ActionResult::ok("get_text", text, 0)
            }
            Err(e) => ActionResult::err("get_text", e.to_string(), 0),
        }
    }

    // ── Tab Management ──

    /// Open a new tab and navigate to URL. Returns the new page.
    pub async fn new_tab(&mut self, url: &str) -> Result<(), BrowserError> {
        let browser = self.browser.as_ref().ok_or_else(|| {
            BrowserError::LaunchFailed("Browser not launched".into())
        })?;
        let page = browser
            .new_page(url)
            .await
            .map_err(|e| BrowserError::NavigationFailed {
                url: url.to_string(),
                reason: e.to_string(),
            })?;
        let _ = page.wait_for_navigation().await;
        eprintln!("hydra-browser: new tab → {url}");
        self.page = Some(Arc::new(Mutex::new(page)));
        Ok(())
    }

    /// Get list of all open pages/tabs.
    pub async fn list_tabs(&self) -> Result<Vec<String>, BrowserError> {
        let browser = self.browser.as_ref().ok_or_else(|| {
            BrowserError::LaunchFailed("Browser not launched".into())
        })?;
        let pages = browser.pages().await.map_err(|e| {
            BrowserError::ActionFailed { action: "list_tabs".into(), reason: e.to_string() }
        })?;
        let mut urls = Vec::new();
        for page in &pages {
            if let Ok(url) = page.url().await {
                urls.push(url.map(|u| u.to_string()).unwrap_or_default());
            }
        }
        Ok(urls)
    }

    /// Switch to a tab by index (0-based).
    pub async fn switch_tab(&mut self, index: usize) -> Result<(), BrowserError> {
        let browser = self.browser.as_ref().ok_or_else(|| {
            BrowserError::LaunchFailed("Browser not launched".into())
        })?;
        let pages = browser.pages().await.map_err(|e| {
            BrowserError::ActionFailed { action: "switch_tab".into(), reason: e.to_string() }
        })?;
        if index >= pages.len() {
            return Err(BrowserError::ActionFailed {
                action: "switch_tab".into(),
                reason: format!("Tab index {index} out of range ({})", pages.len()),
            });
        }
        let page = pages.into_iter().nth(index).unwrap();
        let _ = page.bring_to_front().await;
        eprintln!("hydra-browser: switched to tab {index}");
        self.page = Some(Arc::new(Mutex::new(page)));
        Ok(())
    }

    /// Close the current tab (drops the page reference).
    pub async fn close_tab(&mut self) -> Result<(), BrowserError> {
        if self.page.take().is_some() {
            eprintln!("hydra-browser: tab closed (reference dropped)");
        }
        Ok(())
    }
}

impl Default for BrowserEngine {
    fn default() -> Self {
        Self::new()
    }
}
