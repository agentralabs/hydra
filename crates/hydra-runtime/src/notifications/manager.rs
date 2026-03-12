use std::collections::VecDeque;

use super::types::Notification;

/// In-memory notification manager for Hydra
pub struct NotificationManager {
    queue: VecDeque<Notification>,
    pending_count: usize,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            pending_count: 0,
        }
    }

    /// Send a notification (add to queue)
    pub fn send(&mut self, notification: Notification) {
        if !notification.read {
            self.pending_count += 1;
        }
        self.queue.push_back(notification);
    }

    /// Get the count of unread notifications
    pub fn get_pending_count(&self) -> usize {
        self.pending_count
    }

    /// Get all unread notifications
    pub fn get_unread(&self) -> Vec<&Notification> {
        self.queue.iter().filter(|n| !n.read).collect()
    }

    /// Mark a notification as read by ID
    pub fn mark_read(&mut self, id: &str) -> bool {
        if let Some(notification) = self.queue.iter_mut().find(|n| n.id == id) {
            if !notification.read {
                notification.read = true;
                self.pending_count = self.pending_count.saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    /// Clear all notifications
    pub fn clear_all(&mut self) {
        self.queue.clear();
        self.pending_count = 0;
    }

    /// Get total notification count
    pub fn total_count(&self) -> usize {
        self.queue.len()
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::types::{NotificationUrgency, NotificationAction};

    fn make_notification(id: &str, read: bool) -> Notification {
        Notification {
            id: id.into(),
            title: format!("Title {}", id),
            body: format!("Body {}", id),
            urgency: NotificationUrgency::Normal,
            action: None,
            created_at: chrono::Utc::now(),
            read,
        }
    }

    #[test]
    fn test_new_manager() {
        let mgr = NotificationManager::new();
        assert_eq!(mgr.get_pending_count(), 0);
        assert_eq!(mgr.total_count(), 0);
    }

    #[test]
    fn test_send_unread() {
        let mut mgr = NotificationManager::new();
        mgr.send(make_notification("n1", false));
        assert_eq!(mgr.get_pending_count(), 1);
        assert_eq!(mgr.total_count(), 1);
    }

    #[test]
    fn test_send_already_read() {
        let mut mgr = NotificationManager::new();
        mgr.send(make_notification("n1", true));
        assert_eq!(mgr.get_pending_count(), 0);
        assert_eq!(mgr.total_count(), 1);
    }

    #[test]
    fn test_get_unread() {
        let mut mgr = NotificationManager::new();
        mgr.send(make_notification("n1", false));
        mgr.send(make_notification("n2", true));
        mgr.send(make_notification("n3", false));
        let unread = mgr.get_unread();
        assert_eq!(unread.len(), 2);
    }

    #[test]
    fn test_mark_read() {
        let mut mgr = NotificationManager::new();
        mgr.send(make_notification("n1", false));
        assert_eq!(mgr.get_pending_count(), 1);
        assert!(mgr.mark_read("n1"));
        assert_eq!(mgr.get_pending_count(), 0);
    }

    #[test]
    fn test_mark_read_already_read() {
        let mut mgr = NotificationManager::new();
        mgr.send(make_notification("n1", false));
        mgr.mark_read("n1");
        mgr.mark_read("n1"); // double mark
        assert_eq!(mgr.get_pending_count(), 0);
    }

    #[test]
    fn test_mark_read_nonexistent() {
        let mut mgr = NotificationManager::new();
        assert!(!mgr.mark_read("nonexistent"));
    }

    #[test]
    fn test_clear_all() {
        let mut mgr = NotificationManager::new();
        mgr.send(make_notification("n1", false));
        mgr.send(make_notification("n2", false));
        mgr.clear_all();
        assert_eq!(mgr.total_count(), 0);
        assert_eq!(mgr.get_pending_count(), 0);
    }

    #[test]
    fn test_default() {
        let mgr = NotificationManager::default();
        assert_eq!(mgr.total_count(), 0);
    }

    #[test]
    fn test_notification_with_action() {
        let mut mgr = NotificationManager::new();
        let mut n = make_notification("n1", false);
        n.action = Some(NotificationAction::ApproveRun("run-1".into()));
        mgr.send(n);
        assert_eq!(mgr.total_count(), 1);
    }
}
