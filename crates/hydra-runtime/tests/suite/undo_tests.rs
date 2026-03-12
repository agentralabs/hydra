use hydra_runtime::undo::{GenericAction, UndoError, UndoStack};

fn make_action(desc: &str) -> Box<GenericAction> {
    Box::new(GenericAction::new(desc, desc))
}

#[test]
fn test_undo_stack_push() {
    let mut stack = UndoStack::new(10);
    assert_eq!(stack.undo_count(), 0);
    stack.push(make_action("action1"));
    assert_eq!(stack.undo_count(), 1);
    stack.push(make_action("action2"));
    assert_eq!(stack.undo_count(), 2);
}

#[test]
fn test_undo_single() {
    let mut stack = UndoStack::new(10);
    stack.push(make_action("create file"));
    assert!(stack.can_undo());
    stack.undo().unwrap();
    assert!(!stack.can_undo());
}

#[test]
fn test_redo_single() {
    let mut stack = UndoStack::new(10);
    stack.push(make_action("create file"));
    stack.undo().unwrap();
    assert!(stack.can_redo());
    stack.redo().unwrap();
    assert!(!stack.can_redo());
    assert!(stack.can_undo());
}

#[test]
fn test_undo_redo_sequence() {
    let mut stack = UndoStack::new(10);
    stack.push(make_action("a1"));
    stack.push(make_action("a2"));
    stack.push(make_action("a3"));
    assert_eq!(stack.undo_count(), 3);

    stack.undo().unwrap(); // undo a3
    stack.undo().unwrap(); // undo a2
    assert_eq!(stack.undo_count(), 1);
    assert_eq!(stack.redo_count(), 2);

    stack.redo().unwrap(); // redo a2
    assert_eq!(stack.undo_count(), 2);
    assert_eq!(stack.redo_count(), 1);
}

#[test]
fn test_undo_empty_stack() {
    let mut stack = UndoStack::new(10);
    let result = stack.undo();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), UndoError::NothingToUndo));
}

#[test]
fn test_redo_empty_stack() {
    let mut stack = UndoStack::new(10);
    let result = stack.redo();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), UndoError::NothingToRedo));

    // Also: push then redo with nothing undone
    stack.push(make_action("x"));
    let result = stack.redo();
    assert!(result.is_err());
}

#[test]
fn test_max_size_eviction() {
    let mut stack = UndoStack::new(3);
    stack.push(make_action("a1"));
    stack.push(make_action("a2"));
    stack.push(make_action("a3"));
    assert_eq!(stack.undo_count(), 3);

    // Pushing a 4th should evict the oldest
    stack.push(make_action("a4"));
    assert_eq!(stack.undo_count(), 3);

    // The oldest surviving action should be "a2"
    stack.undo().unwrap(); // undo a4
    stack.undo().unwrap(); // undo a3
    stack.undo().unwrap(); // undo a2
    assert!(!stack.can_undo()); // a1 was evicted
}

#[test]
fn test_can_undo_redo() {
    let mut stack = UndoStack::new(10);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());

    stack.push(make_action("x"));
    assert!(stack.can_undo());
    assert!(!stack.can_redo());

    stack.undo().unwrap();
    assert!(!stack.can_undo());
    assert!(stack.can_redo());
}

#[test]
fn test_action_description() {
    let mut stack = UndoStack::new(10);
    assert!(stack.last_action_description().is_none());

    stack.push(make_action("rename file"));
    assert_eq!(stack.last_action_description(), Some("rename file"));

    stack.push(make_action("delete file"));
    assert_eq!(stack.last_action_description(), Some("delete file"));

    stack.undo().unwrap();
    assert_eq!(stack.last_action_description(), Some("rename file"));
}

#[test]
fn test_undo_clears_redo_on_push() {
    let mut stack = UndoStack::new(10);
    stack.push(make_action("a1"));
    stack.push(make_action("a2"));
    stack.push(make_action("a3"));

    // Undo a3, creating redo history
    stack.undo().unwrap();
    assert!(stack.can_redo());
    assert_eq!(stack.redo_count(), 1);

    // Push a new action — redo history should be cleared
    stack.push(make_action("a4"));
    assert!(!stack.can_redo());
    assert_eq!(stack.redo_count(), 0);
    assert_eq!(stack.undo_count(), 3); // a1, a2, a4
}
