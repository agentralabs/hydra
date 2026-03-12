use chrono::Utc;
use hydra_runtime::notifications::manager::NotificationManager;
use hydra_runtime::notifications::types::{Notification, NotificationAction, NotificationUrgency};

fn make_notification(id: &str, title: &str, urgency: NotificationUrgency) -> Notification {
    Notification {
        id: id.into(),
        title: title.into(),
        body: format!("Body of {title}"),
        urgency,
        action: None,
        created_at: Utc::now(),
        read: false,
    }
}

#[test]
fn test_notification_create() {
    let n = make_notification("n-1", "Test", NotificationUrgency::Normal);
    assert_eq!(n.id, "n-1");
    assert_eq!(n.title, "Test");
    assert_eq!(n.urgency, NotificationUrgency::Normal);
    assert!(!n.read);
}

#[test]
fn test_notification_send() {
    let mut mgr = NotificationManager::new();
    let n = make_notification("n-1", "Alert", NotificationUrgency::High);
    mgr.send(n);
    assert_eq!(mgr.total_count(), 1);
    assert_eq!(mgr.get_pending_count(), 1);
}

#[test]
fn test_notification_queue() {
    let mut mgr = NotificationManager::new();
    mgr.send(make_notification("n-1", "First", NotificationUrgency::Low));
    mgr.send(make_notification("n-2", "Second", NotificationUrgency::Normal));
    mgr.send(make_notification("n-3", "Third", NotificationUrgency::High));

    assert_eq!(mgr.total_count(), 3);
    let unread = mgr.get_unread();
    assert_eq!(unread.len(), 3);
    // Queue order: first in, first out
    assert_eq!(unread[0].id, "n-1");
    assert_eq!(unread[2].id, "n-3");
}

#[test]
fn test_pending_count() {
    let mut mgr = NotificationManager::new();
    assert_eq!(mgr.get_pending_count(), 0);

    mgr.send(make_notification("n-1", "A", NotificationUrgency::Normal));
    mgr.send(make_notification("n-2", "B", NotificationUrgency::Normal));
    assert_eq!(mgr.get_pending_count(), 2);

    mgr.mark_read("n-1");
    assert_eq!(mgr.get_pending_count(), 1);

    // Marking the same one again shouldn't double-decrement
    mgr.mark_read("n-1");
    assert_eq!(mgr.get_pending_count(), 1);
}

#[test]
fn test_mark_read() {
    let mut mgr = NotificationManager::new();
    mgr.send(make_notification("n-1", "Alert", NotificationUrgency::High));

    assert!(mgr.mark_read("n-1"));
    assert!(!mgr.mark_read("nonexistent"));

    let unread = mgr.get_unread();
    assert_eq!(unread.len(), 0);
    assert_eq!(mgr.get_pending_count(), 0);
}

#[test]
fn test_clear_all() {
    let mut mgr = NotificationManager::new();
    mgr.send(make_notification("n-1", "A", NotificationUrgency::Low));
    mgr.send(make_notification("n-2", "B", NotificationUrgency::Normal));
    mgr.send(make_notification("n-3", "C", NotificationUrgency::High));

    mgr.clear_all();
    assert_eq!(mgr.total_count(), 0);
    assert_eq!(mgr.get_pending_count(), 0);
    assert_eq!(mgr.get_unread().len(), 0);
}

#[test]
fn test_urgency_levels() {
    let low = make_notification("n-1", "Low", NotificationUrgency::Low);
    let normal = make_notification("n-2", "Normal", NotificationUrgency::Normal);
    let high = make_notification("n-3", "High", NotificationUrgency::High);

    assert_eq!(low.urgency, NotificationUrgency::Low);
    assert_eq!(normal.urgency, NotificationUrgency::Normal);
    assert_eq!(high.urgency, NotificationUrgency::High);

    // They should all be different
    assert_ne!(low.urgency, normal.urgency);
    assert_ne!(normal.urgency, high.urgency);
}

#[test]
fn test_notification_with_action() {
    let mut mgr = NotificationManager::new();

    let mut n1 = make_notification("n-1", "Open App", NotificationUrgency::Normal);
    n1.action = Some(NotificationAction::OpenApp);

    let mut n2 = make_notification("n-2", "Approve", NotificationUrgency::High);
    n2.action = Some(NotificationAction::ApproveRun("run-42".into()));

    let mut n3 = make_notification("n-3", "Dismiss", NotificationUrgency::Low);
    n3.action = Some(NotificationAction::Dismiss);

    mgr.send(n1);
    mgr.send(n2);
    mgr.send(n3);

    let unread = mgr.get_unread();
    assert_eq!(unread[0].action, Some(NotificationAction::OpenApp));
    assert_eq!(
        unread[1].action,
        Some(NotificationAction::ApproveRun("run-42".into()))
    );
    assert_eq!(unread[2].action, Some(NotificationAction::Dismiss));
}
