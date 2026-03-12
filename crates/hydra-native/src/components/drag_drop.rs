//! Drag-and-drop support for the native UI.

use serde::{Deserialize, Serialize};

/// Which UI region a file was dropped onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropTarget {
    ChatInput,
    EvidencePanel,
}

/// Classified file type based on extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Image,
    Code,
    Text,
    Document,
    Folder,
    Unknown,
}

impl FileType {
    /// Map a file extension (without dot) to a `FileType`.
    pub fn from_extension(ext: &str) -> FileType {
        match ext.to_ascii_lowercase().as_str() {
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => {
                FileType::Image
            }
            // Code
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "c" | "cpp" | "h" | "java"
            | "rb" | "swift" | "kt" | "sh" | "bash" | "zsh" | "toml" | "yaml" | "yml"
            | "json" | "xml" | "html" | "css" | "scss" | "sql" | "zig" | "hs" | "lua"
            | "ex" | "exs" | "erl" => FileType::Code,
            // Text
            "txt" | "md" | "rst" | "log" | "csv" | "tsv" => FileType::Text,
            // Documents
            "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" | "odt" | "rtf" => {
                FileType::Document
            }
            _ => FileType::Unknown,
        }
    }
}

/// A file that was dropped into the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedFile {
    pub path: String,
    pub name: String,
    pub file_type: FileType,
    pub size_bytes: u64,
}

/// State for a drop-zone region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropZoneState {
    pub active: bool,
    pub target: DropTarget,
    pub files: Vec<DroppedFile>,
}

impl DropZoneState {
    /// Create a new inactive drop zone for the given target.
    pub fn new(target: DropTarget) -> Self {
        Self {
            active: false,
            target,
            files: Vec::new(),
        }
    }

    /// Accept dropped file paths, classify them, and add to the file list.
    pub fn accept_drop(&mut self, paths: Vec<String>) {
        for path in paths {
            let name = path
                .rsplit('/')
                .next()
                .or_else(|| path.rsplit('\\').next())
                .unwrap_or(&path)
                .to_string();

            let file_type = name
                .rsplit('.')
                .next()
                .filter(|ext| ext.len() < 10 && !ext.contains('/'))
                .map(FileType::from_extension)
                .unwrap_or(FileType::Unknown);

            self.files.push(DroppedFile {
                path,
                name,
                file_type,
                size_bytes: 0,
            });
        }
        self.active = false;
    }

    /// Remove all dropped files.
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Whether any dropped file is an image.
    pub fn has_images(&self) -> bool {
        self.files.iter().any(|f| f.file_type == FileType::Image)
    }

    /// Whether any dropped file is code.
    pub fn has_code(&self) -> bool {
        self.files.iter().any(|f| f.file_type == FileType::Code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_from_extension() {
        assert_eq!(FileType::from_extension("png"), FileType::Image);
        assert_eq!(FileType::from_extension("rs"), FileType::Code);
        assert_eq!(FileType::from_extension("txt"), FileType::Text);
        assert_eq!(FileType::from_extension("pdf"), FileType::Document);
        assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
        // Case insensitive
        assert_eq!(FileType::from_extension("JPG"), FileType::Image);
        assert_eq!(FileType::from_extension("Py"), FileType::Code);
    }

    #[test]
    fn test_accept_drop_classifies_files() {
        let mut zone = DropZoneState::new(DropTarget::ChatInput);
        zone.accept_drop(vec![
            "/home/user/photo.png".into(),
            "/home/user/main.rs".into(),
            "/home/user/notes.txt".into(),
        ]);
        assert_eq!(zone.files.len(), 3);
        assert_eq!(zone.files[0].file_type, FileType::Image);
        assert_eq!(zone.files[0].name, "photo.png");
        assert_eq!(zone.files[1].file_type, FileType::Code);
        assert_eq!(zone.files[2].file_type, FileType::Text);
    }

    #[test]
    fn test_has_images_and_has_code() {
        let mut zone = DropZoneState::new(DropTarget::EvidencePanel);
        assert!(!zone.has_images());
        assert!(!zone.has_code());

        zone.accept_drop(vec!["/a/b.png".into()]);
        assert!(zone.has_images());
        assert!(!zone.has_code());

        zone.accept_drop(vec!["/a/b.rs".into()]);
        assert!(zone.has_code());
    }

    #[test]
    fn test_clear_removes_all_files() {
        let mut zone = DropZoneState::new(DropTarget::ChatInput);
        zone.accept_drop(vec!["/a.png".into(), "/b.rs".into()]);
        assert_eq!(zone.files.len(), 2);
        zone.clear();
        assert!(zone.files.is_empty());
    }

    #[test]
    fn test_accept_drop_deactivates_zone() {
        let mut zone = DropZoneState::new(DropTarget::ChatInput);
        zone.active = true;
        zone.accept_drop(vec!["/a.txt".into()]);
        assert!(!zone.active);
    }

    #[test]
    fn test_file_name_extraction() {
        let mut zone = DropZoneState::new(DropTarget::ChatInput);
        zone.accept_drop(vec![
            "/deeply/nested/path/file.doc".into(),
            "just_a_file.py".into(),
        ]);
        assert_eq!(zone.files[0].name, "file.doc");
        assert_eq!(zone.files[0].file_type, FileType::Document);
        assert_eq!(zone.files[1].name, "just_a_file.py");
        assert_eq!(zone.files[1].file_type, FileType::Code);
    }
}
