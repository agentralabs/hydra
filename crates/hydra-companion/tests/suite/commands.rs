//! Command parsing tests for companion commands.

use hydra_companion::CompanionCommand;

#[test]
fn parse_digest_command() {
    assert_eq!(CompanionCommand::parse("/digest"), CompanionCommand::Digest);
    assert_eq!(
        CompanionCommand::parse("/DIGEST"),
        CompanionCommand::Digest
    );
}

#[test]
fn parse_inbox_command() {
    assert_eq!(CompanionCommand::parse("/inbox"), CompanionCommand::Inbox);
}

#[test]
fn parse_companion_status_command() {
    assert_eq!(
        CompanionCommand::parse("/companion"),
        CompanionCommand::Status
    );
}

#[test]
fn parse_pause_resume_commands() {
    assert_eq!(CompanionCommand::parse("/pause"), CompanionCommand::Pause);
    assert_eq!(CompanionCommand::parse("/resume"), CompanionCommand::Resume);
}

#[test]
fn parse_later_command() {
    assert_eq!(CompanionCommand::parse("/later"), CompanionCommand::Later);
}

#[test]
fn parse_signal_add_command() {
    let cmd = CompanionCommand::parse("/signal add github");
    assert_eq!(
        cmd,
        CompanionCommand::SignalAdd {
            source: "github".to_string()
        }
    );
}

#[test]
fn parse_signal_mute_command() {
    let cmd = CompanionCommand::parse("/signal mute slack");
    assert_eq!(
        cmd,
        CompanionCommand::SignalMute {
            source: "slack".to_string()
        }
    );
}

#[test]
fn parse_unknown_command() {
    assert_eq!(
        CompanionCommand::parse("hello"),
        CompanionCommand::Unknown
    );
}
