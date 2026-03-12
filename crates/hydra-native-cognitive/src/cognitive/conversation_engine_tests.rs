//! Tests for conversation engine.

use super::*;

#[test]
fn test_conversation_buffer_add_and_limit() {
    let mut buf = ConversationBuffer::new(3);
    buf.add("user", "hello");
    buf.add("assistant", "hi");
    buf.add("user", "how are you");
    buf.add("assistant", "good");
    assert_eq!(buf.turn_count(), 3);
    assert_eq!(buf.to_messages()[0].1, "hi");
}

#[test]
fn test_last_assistant_response() {
    let mut buf = ConversationBuffer::new(15);
    buf.add("user", "hey");
    assert!(buf.last_assistant_response().is_none());
    buf.add("assistant", "yo!");
    assert_eq!(buf.last_assistant_response(), Some("yo!"));
}

#[test]
fn test_format_memories_empty() {
    let result = format_memories_naturally(&[]);
    assert!(result.contains("No stored memories"));
}

#[test]
fn test_format_memories_natural() {
    let mems = vec!["likes rust".into(), "prefers PostgreSQL".into()];
    let result = format_memories_naturally(&mems);
    assert!(result.contains("— likes rust"));
    assert!(result.contains("Weave these"));
    assert!(!result.contains("•"));
}

#[test]
fn test_format_memories_overflow() {
    let mems: Vec<String> = (0..8).map(|i| format!("fact {}", i)).collect();
    let result = format_memories_naturally(&mems);
    assert!(result.contains("3 more things"));
}

#[test]
fn test_build_user_profile_new() {
    let p = build_user_profile(&[], "Alice", 1);
    assert!(p.contains("Alice"));
    assert!(p.contains("new relationship"));
}

#[test]
fn test_build_user_profile_veteran() {
    let p = build_user_profile(&[], "Bob", 200);
    assert!(p.contains("Veteran"));
}

#[test]
fn test_build_user_profile_casual_style() {
    let mems = vec!["they said yo last time".into()];
    let p = build_user_profile(&mems, "Dev", 5);
    assert!(p.contains("casual"));
}

#[test]
fn test_time_context() {
    let tc = build_time_context();
    assert!(!tc.time_of_day.is_empty());
    assert!(!tc.day_of_week.is_empty());
}

#[test]
fn test_emotional_frustration() {
    let buf = ConversationBuffer::new(15);
    let e = detect_emotional_context("ugh this still broken", &buf);
    assert!(e.contains("frustrated"));
}

#[test]
fn test_emotional_excitement() {
    let buf = ConversationBuffer::new(15);
    let e = detect_emotional_context("awesome it works!!", &buf);
    assert!(e.contains("excited"));
}

#[test]
fn test_emotional_confusion() {
    let buf = ConversationBuffer::new(15);
    let e = detect_emotional_context("I don't understand this", &buf);
    assert!(e.contains("confused"));
}

#[test]
fn test_emotional_urgency() {
    let buf = ConversationBuffer::new(15);
    let e = detect_emotional_context("need this today asap", &buf);
    assert!(e.contains("rush"));
}

#[test]
fn test_emotional_casual() {
    let buf = ConversationBuffer::new(15);
    let e = detect_emotional_context("hey", &buf);
    assert!(e.contains("Casual"));
}

#[test]
fn test_emotional_deep_work() {
    let buf = ConversationBuffer::new(15);
    let long_input = "a".repeat(250);
    let e = detect_emotional_context(&long_input, &buf);
    assert!(e.contains("deep work"));
}

#[test]
fn test_emotional_rapid_fire() {
    let mut buf = ConversationBuffer::new(15);
    buf.add("user", "one"); buf.add("user", "two"); buf.add("user", "three");
    let e = detect_emotional_context("four", &buf);
    assert!(e.contains("rapid"));
}

#[test]
fn test_emotional_default() {
    let buf = ConversationBuffer::new(15);
    let e = detect_emotional_context("tell me about rust lifetimes", &buf);
    assert!(e.contains("Normal"));
}

#[test]
fn test_anticipation_tests_pass() {
    let a = generate_anticipation(&None, &[], Some("all tests pass!"));
    assert!(a.is_some());
    assert!(a.unwrap().contains("commit"));
}

#[test]
fn test_anticipation_none() {
    let a = generate_anticipation(&None, &[], Some("here's the info"));
    assert!(a.is_none());
}

#[test]
fn test_context_build() {
    let buf = ConversationBuffer::new(15);
    let ctx = ConversationContext::build(
        "hey there", &buf, &[], "macOS", "Dev", &None, 5,
    );
    assert!(ctx.system_prompt.contains("Hydra"));
    assert!(ctx.system_prompt.contains("Dev"));
    assert!(!ctx.messages.is_empty());
}

#[test]
fn test_context_includes_history() {
    let mut buf = ConversationBuffer::new(15);
    buf.add("user", "old question");
    buf.add("assistant", "old answer");
    let ctx = ConversationContext::build(
        "new question", &buf, &[], "", "User", &None, 1,
    );
    assert_eq!(ctx.messages.len(), 3);
    assert_eq!(ctx.messages[0].1, "old question");
    assert_eq!(ctx.messages[2].1, "new question");
}

#[test]
fn test_system_prompt_template_has_placeholders() {
    assert!(SYSTEM_PROMPT_TEMPLATE.contains("{user_name}"));
    assert!(SYSTEM_PROMPT_TEMPLATE.contains("{user_profile}"));
    assert!(SYSTEM_PROMPT_TEMPLATE.contains("{memory_context}"));
    assert!(SYSTEM_PROMPT_TEMPLATE.contains("{emotional_context}"));
    assert!(SYSTEM_PROMPT_TEMPLATE.contains("{time_of_day}"));
}
