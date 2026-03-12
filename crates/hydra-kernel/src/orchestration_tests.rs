use super::*;

#[test]
fn test_session_new() {
    let session = AgenticSession::new(SessionConfig::default());
    assert_eq!(session.tokens_used(), 0);
    assert_eq!(session.turns_completed(), 0);
    assert!(!session.budget_exhausted());
    assert!(!session.max_turns_reached());
}

#[test]
fn test_session_add_turn() {
    let mut session = AgenticSession::new(SessionConfig::default());
    session.add_turn(TurnRole::User, "hello".into(), 10);
    assert_eq!(session.turns_completed(), 1);
    assert_eq!(session.tokens_used(), 10);
}

#[test]
fn test_session_budget_exhausted() {
    let mut session = AgenticSession::new(SessionConfig {
        total_budget_tokens: 100,
        ..Default::default()
    });
    session.add_turn(TurnRole::User, "task".into(), 50);
    assert!(!session.budget_exhausted());
    session.add_turn(TurnRole::Assistant, "response".into(), 50);
    assert!(session.budget_exhausted());
}

#[test]
fn test_session_max_turns() {
    let mut session = AgenticSession::new(SessionConfig {
        max_turns: 2,
        ..Default::default()
    });
    session.add_turn(TurnRole::User, "a".into(), 5);
    assert!(!session.max_turns_reached());
    session.add_turn(TurnRole::Assistant, "b".into(), 5);
    assert!(session.max_turns_reached());
}

#[test]
fn test_build_messages() {
    let mut session = AgenticSession::new(SessionConfig::default());
    session.add_turn(TurnRole::User, "hello".into(), 5);
    session.add_turn(TurnRole::Assistant, "hi there".into(), 10);
    let msgs = session.build_messages();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].0, "user");
    assert_eq!(msgs[1].0, "assistant");
}

#[test]
fn test_last_answer() {
    let mut session = AgenticSession::new(SessionConfig::default());
    session.add_turn(TurnRole::User, "question".into(), 5);
    assert!(session.last_answer().is_none());
    session.add_turn(TurnRole::Assistant, "answer".into(), 10);
    assert_eq!(session.last_answer(), Some("answer"));
}

#[test]
fn test_is_final_response() {
    assert!(AgenticSession::is_final_response("Here is your answer.", &[]));
    assert!(!AgenticSession::is_final_response("", &[]));
    assert!(!AgenticSession::is_final_response(
        "calling tool",
        &[ToolCall {
            tool_name: "search".into(),
            arguments: serde_json::json!({}),
            call_id: "1".into(),
        }],
    ));
}

#[test]
fn test_session_result_accessors() {
    let result = SessionResult::Complete {
        answer: "done".into(),
        turns: 3,
        tokens: 500,
    };
    assert!(result.is_success());
    assert_eq!(result.answer(), "done");
    assert_eq!(result.turns_used(), 3);
}

#[test]
fn test_should_use_agentic_simple() {
    assert!(!should_use_agentic_session("hey"));
    assert!(!should_use_agentic_session("what time is it?"));
    assert!(!should_use_agentic_session("how are you?"));
}

#[test]
fn test_should_use_agentic_multi_step() {
    assert!(should_use_agentic_session(
        "search for auth code, then understand how it works, then write a test"
    ));
    assert!(should_use_agentic_session(
        "build a REST API with auth and then test it"
    ));
    assert!(should_use_agentic_session("implement this spec"));
}

#[test]
fn test_should_use_agentic_complex_build() {
    assert!(should_use_agentic_session(
        "build a Rust web server with JWT auth and PostgreSQL"
    ));
}

#[test]
fn test_session_elapsed() {
    let mut session = AgenticSession::new(SessionConfig::default());
    assert_eq!(session.elapsed_ms(), 0);
    session.start();
    // Just verify it returns something >= 0 (not testing exact timing)
    assert!(session.elapsed_ms() < 1000);
}

#[test]
fn test_tool_result_becomes_user_message() {
    let mut session = AgenticSession::new(SessionConfig::default());
    session.add_turn(TurnRole::ToolResult, "search results: 5 hits".into(), 20);
    let msgs = session.build_messages();
    assert_eq!(msgs[0].0, "user"); // ToolResult mapped to "user"
}
