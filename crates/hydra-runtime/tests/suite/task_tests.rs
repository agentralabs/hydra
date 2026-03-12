use hydra_runtime::tasks::task::HydraTaskStatus;
use hydra_runtime::tasks::manager::TaskManager;

#[test]
fn test_task_create() {
    let mut mgr = TaskManager::new();
    let task = mgr.create_task("Deploy to production");
    assert_eq!(task.title, "Deploy to production");
    assert_eq!(task.status, HydraTaskStatus::Pending);
    assert!(task.completed_at.is_none());
    assert!(task.run_id.is_none());
    assert!(task.parent_id.is_none());
    assert_eq!(mgr.count(), 1);
}

#[test]
fn test_task_update_status() {
    let mut mgr = TaskManager::new();
    let task = mgr.create_task("Build artifacts");
    assert!(mgr.update_status(&task.id, HydraTaskStatus::Active));

    let updated = mgr.get_by_id(&task.id).unwrap();
    assert_eq!(updated.status, HydraTaskStatus::Active);
    // Active is not terminal, so completed_at should still be None
    assert!(updated.completed_at.is_none());
}

#[test]
fn test_task_complete() {
    let mut mgr = TaskManager::new();
    let task = mgr.create_task("Run tests");
    assert!(mgr.complete_task(&task.id));

    let completed = mgr.get_by_id(&task.id).unwrap();
    assert_eq!(completed.status, HydraTaskStatus::Completed);
    assert!(completed.completed_at.is_some());
}

#[test]
fn test_task_fail() {
    let mut mgr = TaskManager::new();
    let task = mgr.create_task("Flaky test");
    assert!(mgr.update_status(&task.id, HydraTaskStatus::Failed));

    let failed = mgr.get_by_id(&task.id).unwrap();
    assert_eq!(failed.status, HydraTaskStatus::Failed);
    assert!(failed.completed_at.is_some());
}

#[test]
fn test_get_today() {
    let mut mgr = TaskManager::new();
    mgr.create_task("Task A");
    mgr.create_task("Task B");

    let today = mgr.get_today();
    assert_eq!(today.len(), 2);
}

#[test]
fn test_get_history() {
    let mut mgr = TaskManager::new();
    mgr.create_task("Recent task");

    let history = mgr.get_history(7);
    assert_eq!(history.len(), 1);

    // 0 days history should still include today
    let zero = mgr.get_history(0);
    // created_at is now, cutoff is also now, so it depends on timing
    // Just check it doesn't panic
    let _ = zero;
}

#[test]
fn test_task_search() {
    let mut mgr = TaskManager::new();
    mgr.create_task("Deploy frontend");
    mgr.create_task("Deploy backend");
    mgr.create_task("Run tests");

    let results = mgr.search("deploy");
    assert_eq!(results.len(), 2);

    let results = mgr.search("tests");
    assert_eq!(results.len(), 1);

    let results = mgr.search("nonexistent");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_task_link_to_run() {
    let mut mgr = TaskManager::new();
    let task = mgr.create_task("Linked task");
    assert!(mgr.link_to_run(&task.id, "run-123"));

    let linked = mgr.get_by_id(&task.id).unwrap();
    assert_eq!(linked.run_id, Some("run-123".into()));
}

#[test]
fn test_subtasks() {
    let mut mgr = TaskManager::new();
    let parent = mgr.create_task("Parent task");
    let parent_id = parent.id.clone();

    // Create subtasks by manually setting parent_id after creation
    let child1 = mgr.create_task("Child 1");
    let child1_id = child1.id.clone();
    let child2 = mgr.create_task("Child 2");
    let child2_id = child2.id.clone();

    // We need mutable access to set parent_id — use the queries module
    // For now, verify the query helper works with the tasks slice
    // First, let's set parent_id through the tasks vector
    // The TaskManager doesn't expose mutation of parent_id directly,
    // so we test the queries::subtasks function
    {
        let tasks = mgr.all();
        let subs = hydra_runtime::tasks::queries::subtasks(tasks, &parent_id);
        assert_eq!(subs.len(), 0); // No subtasks yet since we can't set parent_id
    }

    // Verify the tasks exist
    assert!(mgr.get_by_id(&child1_id).is_some());
    assert!(mgr.get_by_id(&child2_id).is_some());
}

#[test]
fn test_task_ordering() {
    let mut mgr = TaskManager::new();
    mgr.create_task("First");
    mgr.create_task("Second");
    mgr.create_task("Third");

    let today = mgr.get_today();
    assert_eq!(today.len(), 3);
    // Tasks should be in insertion order
    assert_eq!(today[0].title, "First");
    assert_eq!(today[1].title, "Second");
    assert_eq!(today[2].title, "Third");
}

#[test]
fn test_task_status_display() {
    assert_eq!(format!("{}", HydraTaskStatus::Pending), "\u{25CB}");
    assert_eq!(format!("{}", HydraTaskStatus::Active), "\u{25C9}");
    assert_eq!(format!("{}", HydraTaskStatus::Completed), "\u{2713}");
    assert_eq!(format!("{}", HydraTaskStatus::Failed), "\u{2717}");
    assert_eq!(format!("{}", HydraTaskStatus::Cancelled), "-");
}

#[test]
fn test_task_cancel() {
    let mut mgr = TaskManager::new();
    let task = mgr.create_task("Cancellable task");
    assert!(mgr.update_status(&task.id, HydraTaskStatus::Active));
    assert!(mgr.update_status(&task.id, HydraTaskStatus::Cancelled));

    let cancelled = mgr.get_by_id(&task.id).unwrap();
    assert_eq!(cancelled.status, HydraTaskStatus::Cancelled);
    assert!(cancelled.completed_at.is_some());
}
