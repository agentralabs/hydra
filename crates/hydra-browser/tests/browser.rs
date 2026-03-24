//! Integration tests for hydra-browser.
//! Unit tests are inline in each module.
//! Browser integration tests (marked #[ignore]) require Chrome installed.

use hydra_browser::action::{BrowserAction, ScrollDirection};
use hydra_browser::captcha::{CaptchaSolver, CaptchaType};
use hydra_browser::human::HumanBehavior;
use hydra_browser::login::LoginManager;
use hydra_browser::page::{FormType, PageAnalyzer, PageType};
use hydra_browser::session::{SessionManager, StoredCookie};
use hydra_browser::vision::VisionBudget;

#[test]
fn action_roundtrip() {
    let actions = vec![
        BrowserAction::Navigate {
            url: "https://example.com".into(),
        },
        BrowserAction::Click {
            selector: "#btn".into(),
        },
        BrowserAction::Type {
            selector: "input".into(),
            text: "hello".into(),
        },
        BrowserAction::Scroll {
            direction: ScrollDirection::Down,
            amount: 300,
        },
        BrowserAction::Screenshot,
        BrowserAction::Back,
    ];
    for action in &actions {
        let json = serde_json::to_string(action).unwrap();
        let back: BrowserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, &back);
    }
}

#[test]
fn human_behavior_deterministic_properties() {
    let h = HumanBehavior::new();
    // Bezier always has 21 points
    let pts = h.mouse_curve(0.0, 0.0, 500.0, 500.0);
    assert_eq!(pts.len(), 21);
    // Typing cadence matches string length
    let delays = h.typing_cadence("test input");
    assert_eq!(delays.len(), 10);
}

#[test]
fn page_classifier_comprehensive() {
    assert_eq!(
        PageAnalyzer::classify_page(r#"<html><input type="password">Sign In</html>"#),
        PageType::Login
    );
    assert_eq!(
        PageAnalyzer::classify_page(r#"<html><article>text</article></html>"#),
        PageType::Article
    );
    assert_eq!(
        PageAnalyzer::classify_page(r#"<html><p>normal page</p></html>"#),
        PageType::Unknown
    );
}

#[test]
fn form_detection_comprehensive() {
    // Login form
    let html = r#"<form><input name="username"><input type="password"><button type="submit">Login</button></form>"#;
    let forms = PageAnalyzer::detect_forms(html);
    assert!(forms.iter().any(|f| f.form_type == FormType::Login));

    // Search form
    let html = r#"<form><input name="q" type="search"><button>Search</button></form>"#;
    let forms = PageAnalyzer::detect_forms(html);
    assert!(forms.iter().any(|f| f.form_type == FormType::Search));
}

#[test]
fn captcha_detection_all_types() {
    assert_eq!(
        CaptchaSolver::detect_captcha(r#"<div class="g-recaptcha"></div>"#),
        Some(CaptchaType::ReCaptchaV2)
    );
    assert_eq!(
        CaptchaSolver::detect_captcha(r#"<script src="recaptcha/api.js?render=x"></script>"#),
        Some(CaptchaType::ReCaptchaV3)
    );
    assert_eq!(
        CaptchaSolver::detect_captcha(r#"<div class="h-captcha"></div>"#),
        Some(CaptchaType::HCaptcha)
    );
    assert_eq!(
        CaptchaSolver::detect_captcha(r#"<div class="cf-turnstile"></div>"#),
        Some(CaptchaType::Cloudflare)
    );
    assert_eq!(
        CaptchaSolver::detect_captcha(r#"<html>normal</html>"#),
        None
    );
}

#[test]
fn vision_budget_limits() {
    let budget = VisionBudget::new(5);
    for _ in 0..5 {
        assert!(budget.try_consume());
    }
    assert!(!budget.try_consume());
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn session_stale_check() {
    let fresh = hydra_browser::session::DomainSession {
        domain: "test.com".into(),
        cookies: vec![],
        saved_at: chrono::Utc::now(),
        login_verified: true,
    };
    assert!(!fresh.is_stale());

    let old = hydra_browser::session::DomainSession {
        domain: "test.com".into(),
        cookies: vec![],
        saved_at: chrono::Utc::now() - chrono::Duration::hours(2),
        login_verified: true,
    };
    assert!(old.is_stale());
}

#[test]
fn login_page_detection() {
    assert!(LoginManager::detect_login_page(
        r#"<form><input type="password">Login</form>"#
    ));
    assert!(!LoginManager::detect_login_page(
        r#"<html>Just content</html>"#
    ));
}

#[test]
fn text_extraction() {
    let html = "<div><h1>Title</h1><p>Paragraph <b>bold</b> text</p></div>";
    let text = PageAnalyzer::extract_text(html);
    assert!(text.contains("Title"));
    assert!(text.contains("Paragraph bold text"));
}

// ── Integration tests (require Chrome) ──

#[tokio::test]
#[ignore = "requires Chrome installed"]
async fn browser_launch_and_navigate() {
    let mut engine = hydra_browser::BrowserEngine::new();
    engine.launch().await.expect("Chrome should launch");
    assert!(engine.is_launched());

    engine
        .navigate("https://example.com")
        .await
        .expect("Should navigate");

    let html = engine.html().await.expect("Should get HTML");
    assert!(html.contains("Example Domain"));

    let screenshot = engine.screenshot().await.expect("Should screenshot");
    assert!(!screenshot.is_empty());

    engine.close().await;
    assert!(!engine.is_launched());
}
