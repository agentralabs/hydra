//! Filesystem walker for codebase semantic indexing.
//!
//! Walks a project directory and indexes all Rust source files using
//! hydra-kernel's CodeIndexer, persisting results to hydra-db.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use hydra_db::HydraDb;
use hydra_kernel::code_index::CodeIndexer;

/// Result of indexing a project directory.
#[derive(Debug, Clone)]
pub(crate) struct IndexResult {
    pub files_indexed: usize,
    pub symbols_found: usize,
    pub edges_found: usize,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Walk a project directory and index all Rust source files.
pub(crate) async fn index_project(root: &Path, db: &Arc<HydraDb>) -> IndexResult {
    let start = Instant::now();
    let mut files_indexed = 0usize;
    let mut symbols_found = 0usize;
    let mut edges_found = 0usize;
    let mut errors = Vec::new();

    let rs_files = collect_rs_files(root);
    eprintln!(
        "[hydra:code-index] Found {} .rs files under {}",
        rs_files.len(),
        root.display()
    );

    for file_path in &rs_files {
        match index_single_file(file_path, db) {
            Ok((syms, edgs)) => {
                files_indexed += 1;
                symbols_found += syms;
                edges_found += edgs;
            }
            Err(e) => {
                errors.push(format!("{}: {}", file_path.display(), e));
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    eprintln!(
        "[hydra:code-index] Indexed {} files, {} symbols, {} edges in {}ms ({} errors)",
        files_indexed, symbols_found, edges_found, duration_ms, errors.len()
    );

    IndexResult {
        files_indexed,
        symbols_found,
        edges_found,
        duration_ms,
        errors,
    }
}

/// Index a single file (for incremental updates from file watcher).
pub(crate) fn index_single_file(
    path: &Path,
    db: &HydraDb,
) -> Result<(usize, usize), String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {}", e))?;

    let file_key = path.to_string_lossy().to_string();

    // Extract symbols and edges
    let symbols = CodeIndexer::extract_symbols(&contents);
    let edges = CodeIndexer::extract_edges(&contents, &symbols);

    // Clear old data for this file
    let _ = db.delete_symbols_for_file(&file_key);

    // Persist symbols
    for sym in &symbols {
        let _ = db.upsert_code_symbol(
            &file_key,
            &sym.name,
            sym.symbol_type.as_str(),
            sym.line_number as i64,
            sym.visibility.as_str(),
            sym.signature.as_deref(),
            sym.doc_comment.as_deref(),
        );
    }

    // Persist edges
    for edge in &edges {
        let _ = db.upsert_code_edge(
            &edge.from_symbol,
            &edge.to_symbol,
            edge.edge_type.as_str(),
            &file_key,
        );
    }

    Ok((symbols.len(), edges.len()))
}

// ---------------------------------------------------------------
// Directory walking helpers
// ---------------------------------------------------------------

/// Recursively collect all `.rs` files under `root`, skipping excluded dirs.
fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    collect_recursive(root, &mut results);
    results
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            collect_recursive(&path, out);
        } else if path.is_file() {
            if is_indexable_file(&path) {
                out.push(path);
            }
        }
    }
}

/// Check if a directory should be skipped during walking.
fn should_skip_dir(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return true,
    };
    matches!(name, "target" | ".git" | "node_modules" | ".build" | "out")
}

/// Check if a file is eligible for indexing.
fn is_indexable_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    if ext != Some("rs") {
        return false;
    }
    // Skip files larger than 50KB (likely generated)
    match std::fs::metadata(path) {
        Ok(meta) => meta.len() <= 50_000,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_should_skip_dir() {
        assert!(should_skip_dir(Path::new("/project/target")));
        assert!(should_skip_dir(Path::new("/project/.git")));
        assert!(!should_skip_dir(Path::new("/project/src")));
    }

    #[test]
    fn test_is_indexable_file() {
        let dir = tempfile::tempdir().unwrap();
        let rs_file = dir.path().join("lib.rs");
        fs::write(&rs_file, "pub fn hello() {}").unwrap();
        assert!(is_indexable_file(&rs_file));

        let txt_file = dir.path().join("notes.txt");
        fs::write(&txt_file, "hello").unwrap();
        assert!(!is_indexable_file(&txt_file));
    }

    #[test]
    fn test_collect_rs_files_skips_target() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        let tgt = dir.path().join("target");
        fs::create_dir_all(&tgt).unwrap();
        fs::write(tgt.join("gen.rs"), "// generated").unwrap();

        let files = collect_rs_files(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.rs"));
    }
}
