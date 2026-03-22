//! Task lifecycle, executor, and autonomy level tests.

use hydra_companion::{
    AutonomyLevel, Companion, CompanionTask, SignalItem, SignalRouting, TaskStatus,
};
use hydra_companion::signal::SignalClass;

#[test]
fn task_lifecycle() {
    let mut task = CompanionTask::new("test task".to_string());
    assert!(matches!(task.status, TaskStatus::Pending));
    assert!(!task.is_terminal());

    task.start();
    assert!(task.is_running());
    assert!(task.started_at.is_some());

    task.complete();
    assert!(task.is_terminal());
    assert!(task.completed_at.is_some());
}

#[test]
fn task_failure() {
    let mut task = CompanionTask::new("failing task".to_string());
    task.start();
    task.fail("something broke".to_string());
    assert!(task.is_terminal());
    assert!(matches!(task.status, TaskStatus::Failed { .. }));
}

#[test]
fn task_cancellation() {
    let mut task = CompanionTask::new("cancelled task".to_string());
    task.start();
    task.cancel();
    assert!(task.is_terminal());
    assert!(matches!(task.status, TaskStatus::Cancelled));
}

#[test]
fn task_blocked_status() {
    let mut task = CompanionTask::new("blocked task".to_string());
    task.start();
    task.block("needs auth".to_string());
    assert!(task.is_blocked());
    assert!(!task.is_running());
    assert!(!task.is_terminal());

    task.unblock();
    assert!(task.is_running());
    assert!(!task.is_blocked());
}

#[test]
fn task_status_symbols() {
    let mut task = CompanionTask::new("task".to_string());
    assert_eq!(task.status_symbol(), "⏵");
    task.start();
    assert_eq!(task.status_symbol(), "⏵");
    task.complete();
    assert_eq!(task.status_symbol(), "✓");

    let mut task2 = CompanionTask::new("task2".to_string());
    task2.start();
    task2.fail("err".to_string());
    assert_eq!(task2.status_symbol(), "✗");

    let mut task3 = CompanionTask::new("task3".to_string());
    task3.start();
    task3.block("needs input".to_string());
    assert_eq!(task3.status_symbol(), "⚠");
}

#[test]
fn autonomy_default_is_confirm() {
    assert_eq!(AutonomyLevel::default(), AutonomyLevel::Confirm);
}

#[test]
fn task_with_autonomy() {
    let task = CompanionTask::with_autonomy("auto task".to_string(), AutonomyLevel::Report);
    assert_eq!(task.autonomy, AutonomyLevel::Report);
}

#[test]
fn autonomy_display() {
    assert_eq!(format!("{}", AutonomyLevel::Report), "report");
    assert_eq!(format!("{}", AutonomyLevel::Confirm), "confirm");
    assert_eq!(format!("{}", AutonomyLevel::Summarize), "summarize");
    assert_eq!(format!("{}", AutonomyLevel::Auto), "auto");
}

#[test]
fn task_executor_submit_and_complete() {
    let mut executor = hydra_companion::TaskExecutor::new();
    let id = executor
        .submit("task 1".to_string())
        .expect("submit should succeed");
    assert_eq!(executor.active_count(), 0);

    executor.start_task(id).expect("start should succeed");
    assert_eq!(executor.active_count(), 1);

    executor.complete_task(id).expect("complete should succeed");
    assert_eq!(executor.active_count(), 0);
    assert_eq!(executor.completed_count(), 1);
}

#[test]
fn task_executor_not_found() {
    let mut executor = hydra_companion::TaskExecutor::new();
    let result = executor.start_task(uuid::Uuid::new_v4());
    assert!(result.is_err());
}

#[test]
fn task_executor_block_unblock() {
    let mut executor = hydra_companion::TaskExecutor::new();
    let id = executor.submit("task".to_string()).expect("submit");
    executor.start_task(id).expect("start");
    executor
        .block_task(id, "needs auth".to_string())
        .expect("block");

    let task = executor.get_task(id).expect("get");
    assert!(task.is_blocked());

    executor.unblock_task(id).expect("unblock");
    let task = executor.get_task(id).expect("get");
    assert!(task.is_running());
}

#[test]
fn task_executor_with_autonomy() {
    let mut executor = hydra_companion::TaskExecutor::new();
    let id = executor
        .submit_with_autonomy("task".to_string(), AutonomyLevel::Auto)
        .expect("submit");
    let task = executor.get_task(id).expect("get");
    assert_eq!(task.autonomy, AutonomyLevel::Auto);
}

#[test]
fn companion_signal_and_task_flow() {
    let mut companion = Companion::new();

    let signal = SignalItem::new("kernel".to_string(), "critical error in boot".to_string());
    let routed = companion.receive_signal(signal);
    assert_eq!(routed.class, SignalClass::Urgent);
    assert_eq!(routed.routing, SignalRouting::InterruptNow);
    assert_eq!(companion.signals().len(), 1);

    let task_id = companion
        .submit_task("fix boot".to_string())
        .expect("submit");
    companion.start_task(task_id).expect("start");
    assert_eq!(companion.active_task_count(), 1);

    companion.complete_task(task_id).expect("complete");
    assert_eq!(companion.active_task_count(), 0);
}

#[test]
fn companion_task_limit() {
    let mut companion = Companion::new();
    let mut ids = Vec::new();
    for i in 0..8 {
        let id = companion
            .submit_task(format!("task {i}"))
            .expect("submit should succeed");
        companion.start_task(id).expect("start should succeed");
        ids.push(id);
    }
    let ninth = companion.submit_task("task 8".to_string());
    assert!(ninth.is_err(), "Should reject when all 8 slots are running");
}

#[test]
fn companion_digest() {
    let mut companion = Companion::new();
    let signal = SignalItem::new("email".to_string(), "newsletter arrived".to_string());
    companion.receive_signal(signal);

    let digest = companion.digest();
    assert_eq!(digest.len(), 1);
    let digest2 = companion.digest();
    assert!(digest2.is_empty());
}

#[test]
fn companion_inbox() {
    let mut companion = Companion::new();
    companion.receive_signal(SignalItem::new(
        "gh".to_string(),
        "critical build error".to_string(),
    ));
    companion.receive_signal(SignalItem::new(
        "email".to_string(),
        "newsletter".to_string(),
    ));

    let inbox = companion.inbox();
    assert_eq!(inbox.len(), 2);
}

#[test]
fn companion_pending_urgent_and_notable() {
    let mut companion = Companion::new();

    let urgent = SignalItem::new("sys".to_string(), "critical failure".to_string());
    let notable = SignalItem::new("gh".to_string(), "PR update available".to_string());
    let routine = SignalItem::new("email".to_string(), "newsletter".to_string());

    let r1 = companion.receive_signal(urgent);
    companion.receive_signal(notable);
    companion.receive_signal(routine);

    assert_eq!(companion.pending_urgent().len(), 1);
    assert_eq!(companion.pending_notable().len(), 1);

    companion.mark_surfaced(r1.signal_id);
    assert_eq!(companion.pending_urgent().len(), 0);
}

#[test]
fn companion_pause_resume() {
    let mut companion = Companion::new();
    assert!(!companion.is_paused());
    companion.pause();
    assert!(companion.is_paused());
    companion.resume();
    assert!(!companion.is_paused());
}

#[test]
fn companion_submit_with_autonomy() {
    let mut companion = Companion::new();
    let id = companion
        .submit_task_with_autonomy("monitor repo".to_string(), AutonomyLevel::Report)
        .expect("submit");
    let task = companion.get_task(id).expect("get");
    assert_eq!(task.autonomy, AutonomyLevel::Report);
}
