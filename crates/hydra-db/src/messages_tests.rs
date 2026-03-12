#[cfg(test)]
mod tests {
    use crate::messages::*;
    use crate::store_types::DbError;
    use chrono::Utc;

    fn make_conversation(id: &str) -> Conversation {
        let now = Utc::now().to_rfc3339();
        Conversation {
            id: id.into(),
            title: Some(format!("Conv {}", id)),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn make_message(id: &str, conv_id: &str, role: MessageRole) -> Message {
        Message {
            id: id.into(),
            conversation_id: conv_id.into(),
            role,
            content: format!("Message {}", id),
            created_at: Utc::now().to_rfc3339(),
            run_id: None,
            metadata: None,
        }
    }

    // --- MessageRole ---

    #[test]
    fn test_message_role_as_str() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Hydra.as_str(), "hydra");
    }

    #[test]
    fn test_message_role_from_str() {
        assert_eq!(MessageRole::from_str("user"), Some(MessageRole::User));
        assert_eq!(MessageRole::from_str("hydra"), Some(MessageRole::Hydra));
        assert_eq!(MessageRole::from_str("invalid"), None);
    }

    #[test]
    fn test_message_role_serde() {
        for role in [MessageRole::User, MessageRole::Hydra] {
            let json = serde_json::to_string(&role).unwrap();
            let restored: MessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, role);
        }
    }

    // --- MessageStore Init ---

    #[test]
    fn test_in_memory() {
        let store = MessageStore::in_memory().unwrap();
        let convs = store.list_conversations().unwrap();
        assert!(convs.is_empty());
    }

    #[test]
    fn test_clone_shares_connection() {
        let store = MessageStore::in_memory().unwrap();
        let store2 = store.clone();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let convs = store2.list_conversations().unwrap();
        assert_eq!(convs.len(), 1);
    }

    // --- Conversations ---

    #[test]
    fn test_create_and_get_conversation() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let conv = store.get_conversation_info("c1").unwrap();
        assert_eq!(conv.title, Some("Conv c1".into()));
    }

    #[test]
    fn test_get_conversation_not_found() {
        let store = MessageStore::in_memory().unwrap();
        let err = store.get_conversation_info("nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_list_conversations() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.create_conversation(&make_conversation("c2")).unwrap();
        let convs = store.list_conversations().unwrap();
        assert_eq!(convs.len(), 2);
    }

    #[test]
    fn test_delete_conversation() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.delete_conversation("c1").unwrap();
        assert!(matches!(store.get_conversation_info("c1").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_delete_conversation_not_found() {
        let store = MessageStore::in_memory().unwrap();
        let err = store.delete_conversation("nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    // --- Messages ---

    #[test]
    fn test_add_and_get_message() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let msg = make_message("m1", "c1", MessageRole::User);
        store.add_message(&msg).unwrap();
        let fetched = store.get_message("c1", "m1").unwrap();
        assert_eq!(fetched.content, "Message m1");
        assert_eq!(fetched.role, MessageRole::User);
    }

    #[test]
    fn test_get_message_not_found() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let err = store.get_message("c1", "nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_get_conversation_messages() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.add_message(&make_message("m2", "c1", MessageRole::Hydra)).unwrap();
        let msgs = store.get_conversation("c1", None).unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn test_get_conversation_messages_with_limit() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.add_message(&make_message("m2", "c1", MessageRole::Hydra)).unwrap();
        store.add_message(&make_message("m3", "c1", MessageRole::User)).unwrap();
        let msgs = store.get_conversation("c1", Some(2)).unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn test_get_recent() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.add_message(&make_message("m2", "c1", MessageRole::Hydra)).unwrap();
        let recent = store.get_recent(1).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_search() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let mut msg = make_message("m1", "c1", MessageRole::User);
        msg.content = "hello world".into();
        store.add_message(&msg).unwrap();
        let mut msg2 = make_message("m2", "c1", MessageRole::Hydra);
        msg2.content = "goodbye".into();
        store.add_message(&msg2).unwrap();
        let results = store.search("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[test]
    fn test_search_no_results() {
        let store = MessageStore::in_memory().unwrap();
        let results = store.search("nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_message_with_metadata() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let mut msg = make_message("m1", "c1", MessageRole::User);
        msg.metadata = Some(serde_json::json!({"key": "value"}));
        store.add_message(&msg).unwrap();
        let fetched = store.get_message("c1", "m1").unwrap();
        assert!(fetched.metadata.is_some());
        assert_eq!(fetched.metadata.unwrap()["key"], "value");
    }

    #[test]
    fn test_message_with_run_id() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let mut msg = make_message("m1", "c1", MessageRole::User);
        msg.run_id = Some("run-123".into());
        store.add_message(&msg).unwrap();
        let fetched = store.get_message("c1", "m1").unwrap();
        assert_eq!(fetched.run_id, Some("run-123".into()));
    }

    #[test]
    fn test_delete_conversation_cascades_messages() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.delete_conversation("c1").unwrap();
        let msgs = store.get_conversation("c1", None).unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_conversation_no_title() {
        let store = MessageStore::in_memory().unwrap();
        let mut conv = make_conversation("c1");
        conv.title = None;
        store.create_conversation(&conv).unwrap();
        let fetched = store.get_conversation_info("c1").unwrap();
        assert!(fetched.title.is_none());
    }

    // --- Serde ---

    #[test]
    fn test_message_serde() {
        let msg = make_message("m1", "c1", MessageRole::User);
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "m1");
    }

    #[test]
    fn test_conversation_serde() {
        let conv = make_conversation("c1");
        let json = serde_json::to_string(&conv).unwrap();
        let restored: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "c1");
    }
}
