use super::action::{UndoError, UndoableAction};

/// A bounded undo/redo stack
pub struct UndoStack {
    actions: Vec<Box<dyn UndoableAction>>,
    /// Points to the next slot for a new action (== number of undoable actions)
    current: usize,
    max_size: usize,
}

impl UndoStack {
    /// Create a new stack with the given maximum capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            actions: Vec::new(),
            current: 0,
            max_size,
        }
    }

    /// Push a new action onto the stack.
    /// This clears any redo history and evicts the oldest action if at capacity.
    pub fn push(&mut self, action: Box<dyn UndoableAction>) {
        // Truncate any redo history
        self.actions.truncate(self.current);

        // Evict oldest if at capacity
        if self.actions.len() >= self.max_size {
            self.actions.remove(0);
            self.current = self.current.saturating_sub(1);
        }

        self.actions.push(action);
        self.current = self.actions.len();
    }

    /// Undo the most recent action
    pub fn undo(&mut self) -> Result<(), UndoError> {
        if self.current == 0 {
            return Err(UndoError::NothingToUndo);
        }
        self.current -= 1;
        self.actions[self.current].undo()
    }

    /// Redo the most recently undone action
    pub fn redo(&mut self) -> Result<(), UndoError> {
        if self.current >= self.actions.len() {
            return Err(UndoError::NothingToRedo);
        }
        let result = self.actions[self.current].redo();
        self.current += 1;
        result
    }

    /// Whether there is anything to undo
    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    /// Whether there is anything to redo
    pub fn can_redo(&self) -> bool {
        self.current < self.actions.len()
    }

    /// Description of the last undoable action, if any
    pub fn last_action_description(&self) -> Option<&str> {
        if self.current > 0 {
            Some(self.actions[self.current - 1].description())
        } else {
            None
        }
    }

    /// Number of actions that can be undone
    pub fn undo_count(&self) -> usize {
        self.current
    }

    /// Number of actions that can be redone
    pub fn redo_count(&self) -> usize {
        self.actions.len() - self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::action::GenericAction;

    fn action(id: &str) -> Box<dyn UndoableAction> {
        Box::new(GenericAction::new(id, format!("Action {}", id)))
    }

    #[test]
    fn test_new_stack_empty() {
        let stack = UndoStack::new(10);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
        assert!(stack.last_action_description().is_none());
    }

    #[test]
    fn test_push_and_undo() {
        let mut stack = UndoStack::new(10);
        stack.push(action("1"));
        assert!(stack.can_undo());
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.last_action_description(), Some("Action 1"));
        stack.undo().unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut stack = UndoStack::new(10);
        stack.push(action("1"));
        stack.undo().unwrap();
        assert!(stack.can_redo());
        stack.redo().unwrap();
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_nothing_to_undo() {
        let mut stack = UndoStack::new(10);
        assert!(matches!(stack.undo(), Err(UndoError::NothingToUndo)));
    }

    #[test]
    fn test_nothing_to_redo() {
        let mut stack = UndoStack::new(10);
        assert!(matches!(stack.redo(), Err(UndoError::NothingToRedo)));
    }

    #[test]
    fn test_push_clears_redo_history() {
        let mut stack = UndoStack::new(10);
        stack.push(action("1"));
        stack.push(action("2"));
        stack.undo().unwrap();
        assert!(stack.can_redo());
        stack.push(action("3"));
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 2);
    }

    #[test]
    fn test_max_size_eviction() {
        let mut stack = UndoStack::new(3);
        stack.push(action("1"));
        stack.push(action("2"));
        stack.push(action("3"));
        assert_eq!(stack.undo_count(), 3);
        stack.push(action("4"));
        assert_eq!(stack.undo_count(), 3);
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut stack = UndoStack::new(10);
        stack.push(action("1"));
        stack.push(action("2"));
        stack.push(action("3"));
        assert_eq!(stack.undo_count(), 3);
        stack.undo().unwrap();
        stack.undo().unwrap();
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 2);
        stack.redo().unwrap();
        assert_eq!(stack.undo_count(), 2);
        assert_eq!(stack.redo_count(), 1);
    }
}
