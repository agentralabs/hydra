use hydra_db::*;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn make_conversation(id: &str) -> Conversation {
    let ts = now();
    Conversation {
        id: id.into(),
        title: Some(format!("Conversation {id}")),
        created_at: ts.clone(),
        updated_at: ts,
    }
}

fn make_message(id: &str, conversation_id: &str, role: MessageRole, content: &str) -> Message {
    Message {
        id: id.into(),
        conversation_id: conversation_id.into(),
        role,
        content: content.into(),
        created_at: now(),
        run_id: None,
        metadata: None,
    }
}

#[test]
fn test_message_create() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    let msg = make_message("msg-1", "conv-1", MessageRole::User, "Hello");
    store.add_message(&msg).unwrap();

    let messages = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "msg-1");
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[0].role, MessageRole::User);
}

#[test]
fn test_message_get_conversation() {
    let store = MessageStore::in_memory().unwrap();
    let conv1 = make_conversation("conv-1");
    let conv2 = make_conversation("conv-2");
    store.create_conversation(&conv1).unwrap();
    store.create_conversation(&conv2).unwrap();

    store
        .add_message(&make_message(
            "msg-1",
            "conv-1",
            MessageRole::User,
            "Hello",
        ))
        .unwrap();
    store
        .add_message(&make_message(
            "msg-2",
            "conv-1",
            MessageRole::Hydra,
            "Hi there!",
        ))
        .unwrap();
    store
        .add_message(&make_message(
            "msg-3",
            "conv-2",
            MessageRole::User,
            "Other convo",
        ))
        .unwrap();

    let conv1_msgs = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(conv1_msgs.len(), 2);

    let conv2_msgs = store.get_conversation("conv-2", None).unwrap();
    assert_eq!(conv2_msgs.len(), 1);
}

#[test]
fn test_message_ordering() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    // Insert with explicit timestamps to ensure ordering
    for i in 0..5 {
        let mut msg = make_message(
            &format!("msg-{i}"),
            "conv-1",
            MessageRole::User,
            &format!("Message {i}"),
        );
        msg.created_at =
            chrono::Utc::now().to_rfc3339();
        store.add_message(&msg).unwrap();
    }

    let messages = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(messages.len(), 5);
    // Messages should be ordered by created_at ASC
    for i in 0..5 {
        assert_eq!(messages[i].content, format!("Message {i}"));
    }
}

#[test]
fn test_message_with_run_id() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    let mut msg = make_message("msg-1", "conv-1", MessageRole::Hydra, "Running task...");
    msg.run_id = Some("run-abc-123".into());
    store.add_message(&msg).unwrap();

    let messages = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].run_id, Some("run-abc-123".into()));
}

#[test]
fn test_conversation_create() {
    let store = MessageStore::in_memory().unwrap();
    let conv = Conversation {
        id: "conv-42".into(),
        title: Some("My Conversation".into()),
        created_at: now(),
        updated_at: now(),
    };
    store.create_conversation(&conv).unwrap();

    let fetched = store.get_conversation_info("conv-42").unwrap();
    assert_eq!(fetched.id, "conv-42");
    assert_eq!(fetched.title, Some("My Conversation".into()));
}

#[test]
fn test_recent_messages() {
    let store = MessageStore::in_memory().unwrap();
    let conv1 = make_conversation("conv-1");
    let conv2 = make_conversation("conv-2");
    store.create_conversation(&conv1).unwrap();
    store.create_conversation(&conv2).unwrap();

    for i in 0..5 {
        let conv_id = if i % 2 == 0 { "conv-1" } else { "conv-2" };
        store
            .add_message(&make_message(
                &format!("msg-{i}"),
                conv_id,
                MessageRole::User,
                &format!("Message {i}"),
            ))
            .unwrap();
    }

    let recent = store.get_recent(3).unwrap();
    assert_eq!(recent.len(), 3);
    // Most recent first
    assert_eq!(recent[0].id, "msg-4");
}

#[test]
fn test_search_messages() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    store
        .add_message(&make_message(
            "msg-1",
            "conv-1",
            MessageRole::User,
            "Deploy the application",
        ))
        .unwrap();
    store
        .add_message(&make_message(
            "msg-2",
            "conv-1",
            MessageRole::Hydra,
            "Deploying now...",
        ))
        .unwrap();
    store
        .add_message(&make_message(
            "msg-3",
            "conv-1",
            MessageRole::User,
            "Check the logs",
        ))
        .unwrap();

    let results = store.search("deploy").unwrap();
    // LIKE is case-insensitive on ASCII in SQLite by default
    assert!(results.len() >= 1);
    assert!(results.iter().any(|m| m.id == "msg-1"));
}

#[test]
fn test_delete_conversation() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    store
        .add_message(&make_message(
            "msg-1",
            "conv-1",
            MessageRole::User,
            "Hello",
        ))
        .unwrap();
    store
        .add_message(&make_message(
            "msg-2",
            "conv-1",
            MessageRole::Hydra,
            "World",
        ))
        .unwrap();

    store.delete_conversation("conv-1").unwrap();

    // Conversation should be gone
    let result = store.get_conversation_info("conv-1");
    assert!(result.is_err());

    // Messages should be cascade-deleted
    let messages = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(messages.len(), 0);
}

#[test]
fn test_message_metadata() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    let mut msg = make_message("msg-1", "conv-1", MessageRole::User, "With metadata");
    msg.metadata = Some(serde_json::json!({
        "source": "cli",
        "tokens": 42
    }));
    store.add_message(&msg).unwrap();

    let messages = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(messages.len(), 1);
    let meta = messages[0].metadata.as_ref().unwrap();
    assert_eq!(meta["source"], "cli");
    assert_eq!(meta["tokens"], 42);
}

#[test]
fn test_conversation_pagination() {
    let store = MessageStore::in_memory().unwrap();
    let conv = make_conversation("conv-1");
    store.create_conversation(&conv).unwrap();

    for i in 0..20 {
        store
            .add_message(&make_message(
                &format!("msg-{i}"),
                "conv-1",
                MessageRole::User,
                &format!("Message {i}"),
            ))
            .unwrap();
    }

    // Get only first 5
    let page = store.get_conversation("conv-1", Some(5)).unwrap();
    assert_eq!(page.len(), 5);
    assert_eq!(page[0].content, "Message 0");
    assert_eq!(page[4].content, "Message 4");

    // Get all
    let all = store.get_conversation("conv-1", None).unwrap();
    assert_eq!(all.len(), 20);
}
