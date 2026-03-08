use std::path::{Path, PathBuf};

/// Errors from undo/redo operations
#[derive(Debug)]
pub enum UndoError {
    /// Nothing to undo
    NothingToUndo,
    /// Nothing to redo
    NothingToRedo,
    /// The undo/redo operation itself failed
    OperationFailed(String),
    /// I/O error during file-based undo
    Io(std::io::Error),
}

impl std::fmt::Display for UndoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NothingToUndo => write!(f, "nothing to undo"),
            Self::NothingToRedo => write!(f, "nothing to redo"),
            Self::OperationFailed(msg) => write!(f, "undo operation failed: {}", msg),
            Self::Io(e) => write!(f, "I/O error during undo: {}", e),
        }
    }
}

impl std::error::Error for UndoError {}

/// Trait for actions that can be undone and redone
pub trait UndoableAction: Send + Sync {
    /// Human-readable description of this action
    fn description(&self) -> &str;

    /// Reverse this action
    fn undo(&mut self) -> Result<(), UndoError>;

    /// Re-apply this action after it was undone
    fn redo(&mut self) -> Result<(), UndoError>;
}

/// Tracks creation of a file so it can be undone (deleted) and redone (recreated)
pub struct FileCreateAction {
    path: PathBuf,
    content: Vec<u8>,
    description: String,
}

impl FileCreateAction {
    pub fn new(path: impl Into<PathBuf>, content: Vec<u8>) -> Self {
        let path = path.into();
        let desc = format!("Create file {}", path.display());
        Self {
            path,
            content,
            description: desc,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl UndoableAction for FileCreateAction {
    fn description(&self) -> &str {
        &self.description
    }

    fn undo(&mut self) -> Result<(), UndoError> {
        // Read current content before deleting (in case it was modified)
        if let Ok(current) = std::fs::read(&self.path) {
            self.content = current;
        }
        std::fs::remove_file(&self.path).map_err(UndoError::Io)
    }

    fn redo(&mut self) -> Result<(), UndoError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(UndoError::Io)?;
        }
        std::fs::write(&self.path, &self.content).map_err(UndoError::Io)
    }
}

/// A generic undoable action identified by a description and unique id.
/// Since closures are hard to make Send+Sync, this stores only metadata
/// and delegates actual undo/redo to an external handler.
pub struct GenericAction {
    id: String,
    description: String,
    undone: bool,
}

impl GenericAction {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            undone: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_undone(&self) -> bool {
        self.undone
    }
}

impl UndoableAction for GenericAction {
    fn description(&self) -> &str {
        &self.description
    }

    fn undo(&mut self) -> Result<(), UndoError> {
        self.undone = true;
        Ok(())
    }

    fn redo(&mut self) -> Result<(), UndoError> {
        self.undone = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_action_new() {
        let action = GenericAction::new("id-1", "Test action");
        assert_eq!(action.id(), "id-1");
        assert_eq!(action.description(), "Test action");
        assert!(!action.is_undone());
    }

    #[test]
    fn test_generic_action_undo_redo() {
        let mut action = GenericAction::new("id-1", "Test");
        assert!(!action.is_undone());
        action.undo().unwrap();
        assert!(action.is_undone());
        action.redo().unwrap();
        assert!(!action.is_undone());
    }

    #[test]
    fn test_file_create_action_path() {
        let action = FileCreateAction::new("/tmp/test.txt", b"hello".to_vec());
        assert_eq!(action.path(), Path::new("/tmp/test.txt"));
        assert_eq!(action.description(), "Create file /tmp/test.txt");
    }

    #[test]
    fn test_undo_error_display() {
        assert_eq!(UndoError::NothingToUndo.to_string(), "nothing to undo");
        assert_eq!(UndoError::NothingToRedo.to_string(), "nothing to redo");
        let op_err = UndoError::OperationFailed("test".into());
        assert!(op_err.to_string().contains("test"));
    }
}
