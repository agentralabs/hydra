//! hydra-browser test harness — validates all browser components.
//! Run: cargo run -p hydra-browser --bin browser-harness

use hydra_browser::action::BrowserAction;
use hydra_browser::captcha::{CaptchaSolver, CaptchaType};
use hydra_browser::computer_use::ComputerUseAgent;
use hydra_browser::human::HumanBehavior;
use hydra_browser::login::LoginManager;
use hydra_browser::page::{FormType, PageAnalyzer, PageType};
use hydra_browser::session::SessionManager;
use hydra_browser::vision::VisionBudget;

struct Test {
    name: &'static str,
    passed: bool,
    notes: String,
}

fn main() {
    println!("=== hydra-browser test harness ===");
    println!("Phase: HANDS  Layer: Browser Automation\n");

    let mut tests: Vec<Test> = Vec::new();

    // ── Action Tests ──
    {
        let action = BrowserAction::Click { selector: "#btn".into() };
        let json = serde_json::to_string(&action).unwrap();
        let back: BrowserAction = serde_json::from_str(&json).unwrap();
        tests.push(Test {
            name: "action_serialization_roundtrip",
            passed: action == back,
            notes: format!("json: {json}"),
        });
    }
    {
        tests.push(Test {
            name: "mutation_detection",
            passed: BrowserAction::Click { selector: "x".into() }.is_mutation()
                && !BrowserAction::Screenshot.is_mutation(),
            notes: "click=mutation, screenshot=read-only".into(),
        });
    }

    // ── Human Behavior Tests ──
    {
        let h = HumanBehavior::new();
        let delays: Vec<u64> = (0..100).map(|_| h.random_delay_ms()).collect();
        let all_in_range = delays.iter().all(|d| *d >= 80 && *d <= 350);
        tests.push(Test {
            name: "human_delay_range",
            passed: all_in_range,
            notes: format!("min={} max={}", delays.iter().min().unwrap(), delays.iter().max().unwrap()),
        });
    }
    {
        let h = HumanBehavior::new();
        let points = h.mouse_curve(0.0, 0.0, 100.0, 200.0);
        let starts_ok = (points[0].0).abs() < 0.01 && (points[0].1).abs() < 0.01;
        let last = points.last().unwrap();
        let ends_ok = (last.0 - 100.0).abs() < 0.01 && (last.1 - 200.0).abs() < 0.01;
        tests.push(Test {
            name: "bezier_curve_endpoints",
            passed: starts_ok && ends_ok && points.len() == 21,
            notes: format!("{} points", points.len()),
        });
    }
    {
        let h = HumanBehavior::new();
        let cadence = h.typing_cadence("Hello!");
        tests.push(Test {
            name: "typing_cadence_length",
            passed: cadence.len() == 6,
            notes: format!("{} chars → {} delays", 6, cadence.len()),
        });
    }

    // ── Page Analysis Tests ──
    {
        let html = r#"<form><input name="email"><input type="password"><button>Login</button></form>"#;
        let forms = PageAnalyzer::detect_forms(html);
        let has_login = forms.iter().any(|f| f.form_type == FormType::Login);
        tests.push(Test {
            name: "detect_login_form",
            passed: has_login,
            notes: format!("{} forms detected", forms.len()),
        });
    }
    {
        let html = "<html><body><article>Content</article></body></html>";
        let page_type = PageAnalyzer::classify_page(html);
        tests.push(Test {
            name: "classify_article_page",
            passed: page_type == PageType::Article,
            notes: format!("{:?}", page_type),
        });
    }
    {
        let html = "<p>Hello <b>world</b>!</p>";
        let text = PageAnalyzer::extract_text(html);
        tests.push(Test {
            name: "extract_text_strips_html",
            passed: text.contains("Hello world!"),
            notes: format!("extracted: '{}'", &text[..text.len().min(50)]),
        });
    }

    // ── Session Tests ──
    {
        let mut mgr = SessionManager::new();
        // Can't test disk I/O in harness easily, test in-memory logic
        let has = mgr.has_valid_session("nonexistent.com");
        tests.push(Test {
            name: "session_miss_returns_false",
            passed: !has,
            notes: "no session for unknown domain".into(),
        });
    }

    // ── Login Detection Tests ──
    {
        let html = r#"<form><input type="password"><button>Sign In</button></form>"#;
        let is_login = LoginManager::detect_login_page(html);
        tests.push(Test {
            name: "detect_login_page",
            passed: is_login,
            notes: "password field + sign in button".into(),
        });
    }
    {
        let html = "<html><body><h1>Welcome</h1></body></html>";
        let is_login = LoginManager::detect_login_page(html);
        tests.push(Test {
            name: "non_login_page_rejected",
            passed: !is_login,
            notes: "no password field".into(),
        });
    }

    // ── CAPTCHA Detection Tests ──
    {
        let html = r#"<div class="g-recaptcha" data-sitekey="abc"></div>"#;
        let captcha = CaptchaSolver::detect_captcha(html);
        tests.push(Test {
            name: "detect_recaptcha_v2",
            passed: captcha == Some(CaptchaType::ReCaptchaV2),
            notes: format!("{:?}", captcha),
        });
    }
    {
        let html = "<html><body>Normal page</body></html>";
        let captcha = CaptchaSolver::detect_captcha(html);
        tests.push(Test {
            name: "no_captcha_on_normal_page",
            passed: captcha.is_none(),
            notes: "no captcha detected".into(),
        });
    }

    // ── Vision Budget Tests ──
    {
        let budget = VisionBudget::new(3);
        let c1 = budget.try_consume();
        let c2 = budget.try_consume();
        let c3 = budget.try_consume();
        let c4 = budget.try_consume(); // should fail
        tests.push(Test {
            name: "vision_budget_enforcement",
            passed: c1 && c2 && c3 && !c4,
            notes: format!("remaining={}", budget.remaining()),
        });
    }

    // ── Computer Use Agent Tests ──
    {
        let _agent = ComputerUseAgent::new();
        tests.push(Test {
            name: "computer_use_agent_created",
            passed: true,
            notes: "agent initialized with default max_steps".into(),
        });
    }

    // ── Results ──
    println!();
    let mut passed = 0;
    let mut failed = 0;
    for t in &tests {
        let status = if t.passed { "PASS" } else { "FAIL" };
        println!("  [{status}] {} — {}", t.name, t.notes);
        if t.passed {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!("\n=== Results: {passed}/{} passed, {failed} failed ===", tests.len());
    if failed > 0 {
        std::process::exit(1);
    }
    println!("Phase HANDS — Browser Automation: COMPLETE");
}
