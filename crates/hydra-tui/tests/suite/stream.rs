//! Stream item and conversation stream tests.

use hydra_tui::dot::DotKind;
use hydra_tui::stream::ConversationStream;
use hydra_tui::stream_types::{BriefingPriority, CompanionStatus, StreamItem};

#[test]
fn stream_push_and_render() {
    let mut stream = ConversationStream::new();
    stream.push(StreamItem::UserMessage {
        id: uuid::Uuid::new_v4(),
        text: "Hello".to_string(),
        timestamp: chrono::Utc::now(),
    });
    stream.push(StreamItem::AssistantText {
        id: uuid::Uuid::new_v4(),
        text: "Hi there".to_string(),
        timestamp: chrono::Utc::now(),
    });
    assert_eq!(stream.len(), 2);
    let lines = stream.to_lines(10);
    assert_eq!(lines.len(), 2);
}

#[test]
fn stream_scrolling() {
    let mut stream = ConversationStream::new();
    for i in 0..20 {
        stream.push(StreamItem::SystemNotification {
            id: uuid::Uuid::new_v4(),
            content: format!("Notification {i}"),
            timestamp: chrono::Utc::now(),
        });
    }
    stream.scroll_up(5);
    assert_eq!(stream.scroll_offset(), 5);
    stream.scroll_down(3);
    assert_eq!(stream.scroll_offset(), 2);
    stream.scroll_to_bottom();
    assert_eq!(stream.scroll_offset(), 0);
}

#[test]
fn stream_item_kinds() {
    let items = vec![
        StreamItem::Blank,
        StreamItem::ToolDot {
            id: uuid::Uuid::new_v4(),
            tool_name: "test".to_string(),
            kind: DotKind::Active,
            timestamp: chrono::Utc::now(),
        },
        StreamItem::ToolConnector {
            id: uuid::Uuid::new_v4(),
            label: "pipe".to_string(),
        },
        StreamItem::Truncation {
            id: uuid::Uuid::new_v4(),
            chars_truncated: 100,
        },
        StreamItem::BeliefCitation {
            id: uuid::Uuid::new_v4(),
            belief: "test belief".to_string(),
            confidence: 0.9,
        },
        StreamItem::CompanionTask {
            id: uuid::Uuid::new_v4(),
            description: "task".to_string(),
            status: CompanionStatus::Running,
            timestamp: chrono::Utc::now(),
        },
        StreamItem::BriefingItem {
            id: uuid::Uuid::new_v4(),
            content: "briefing".to_string(),
            priority: BriefingPriority::High,
            timestamp: chrono::Utc::now(),
        },
        StreamItem::DreamNotification {
            id: uuid::Uuid::new_v4(),
            content: "dream".to_string(),
            timestamp: chrono::Utc::now(),
        },
    ];
    let mut stream = ConversationStream::new();
    for item in items {
        stream.push(item);
    }
    let lines = stream.to_lines(20);
    assert_eq!(lines.len(), 8);
}
