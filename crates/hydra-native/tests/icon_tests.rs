use hydra_native::components::icon::IconState;

#[test]
fn test_icon_state_default_is_idle() {
    assert_eq!(IconState::default(), IconState::Idle);
}

#[test]
fn test_icon_idle_css_class() {
    assert_eq!(IconState::Idle.css_class(), "icon-idle");
}

#[test]
fn test_icon_listening_css_class() {
    assert_eq!(IconState::Listening.css_class(), "icon-listening");
}

#[test]
fn test_icon_working_css_class() {
    assert_eq!(IconState::Working.css_class(), "icon-working");
}

#[test]
fn test_icon_needs_attention_css_class() {
    assert_eq!(IconState::NeedsAttention.css_class(), "icon-needs-attention");
}

#[test]
fn test_icon_approval_needed_css_class() {
    assert_eq!(IconState::ApprovalNeeded.css_class(), "icon-approval");
}

#[test]
fn test_icon_success_css_class() {
    assert_eq!(IconState::Success.css_class(), "icon-success");
}

#[test]
fn test_icon_error_css_class() {
    assert_eq!(IconState::Error.css_class(), "icon-error");
}

#[test]
fn test_icon_offline_css_class() {
    assert_eq!(IconState::Offline.css_class(), "icon-offline");
}

#[test]
fn test_icon_offline_is_hollow() {
    assert!(IconState::Offline.is_hollow());
    // All other states should not be hollow
    assert!(!IconState::Idle.is_hollow());
    assert!(!IconState::Listening.is_hollow());
    assert!(!IconState::Working.is_hollow());
    assert!(!IconState::NeedsAttention.is_hollow());
    assert!(!IconState::ApprovalNeeded.is_hollow());
    assert!(!IconState::Success.is_hollow());
    assert!(!IconState::Error.is_hollow());
}

#[test]
fn test_icon_colors() {
    assert_eq!(IconState::Idle.color(), "#4A9EFF");
    assert_eq!(IconState::Listening.color(), "#6366F1");
    assert_eq!(IconState::Working.color(), "#6366F1");
    assert_eq!(IconState::NeedsAttention.color(), "#FFAA4A");
    assert_eq!(IconState::ApprovalNeeded.color(), "#FFAA4A");
    assert_eq!(IconState::Success.color(), "#4ADE80");
    assert_eq!(IconState::Error.color(), "#FF6B6B");
    assert_eq!(IconState::Offline.color(), "#9CA3AF");
}

#[test]
fn test_icon_labels() {
    assert_eq!(IconState::Idle.label(), "Ready");
    assert_eq!(IconState::Listening.label(), "Listening...");
    assert_eq!(IconState::Working.label(), "Working...");
    assert_eq!(IconState::NeedsAttention.label(), "Attention needed");
    assert_eq!(IconState::ApprovalNeeded.label(), "Approval needed");
    assert_eq!(IconState::Success.label(), "Done!");
    assert_eq!(IconState::Error.label(), "Something went wrong");
    assert_eq!(IconState::Offline.label(), "Offline");
}
