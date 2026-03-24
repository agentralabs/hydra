//! hydra-browser — browser automation engine for Hydra.
//!
//! Provides headless Chrome control via CDP (Chrome DevTools Protocol),
//! with human-like behavior, login automation, CAPTCHA solving,
//! and a computer-use agent for multi-step tasks.

pub mod action;
pub mod captcha;
pub mod computer_use;
pub mod constants;
pub mod engine;
pub mod errors;
pub mod fingerprint;
pub mod human;
pub mod limiter;
pub mod login;
pub mod page;
pub mod pool;
pub mod session;
pub mod vision;
pub mod warmup;

// ── Re-exports ──

pub use action::{ActionResult, BrowserAction, ScrollDirection};
pub use captcha::{CaptchaResult, CaptchaSolver, CaptchaType};
pub use computer_use::{ComputerUseAgent, ComputerUseStep, TaskResult};
pub use engine::BrowserEngine;
pub use errors::BrowserError;
pub use human::HumanBehavior;
pub use login::{LoginCredentials, LoginManager, LoginResult};
pub use page::{DetectedForm, FormField, FormType, PageAnalyzer, PageElement, PageType};
pub use session::{DomainSession, SessionManager, StoredCookie};
pub use vision::{BudgetedVision, VisionBudget, VisionProvider};
pub use fingerprint::BrowserProfile;
pub use limiter::{RateLimitStatus, RateLimiter};
pub use warmup::WarmupStatus;
