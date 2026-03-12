//! Smart patch applicator — avoids duplicates, registers modules, deduplicates imports.

use std::fs;
use std::path::Path;

/// Apply code to a target file intelligently.
/// - New file: write content, register `pub mod` in parent lib.rs/mod.rs
/// - Existing file: skip if function already exists, otherwise append
pub fn apply_smart(project_dir: &Path, target_file: &str, code: &str) -> Result<(), String> {
    let file_path = project_dir.join(target_file);
    if file_path.exists() {
        let existing = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        // Extract function names from the new code and skip if all already exist
        let new_fns = extract_fn_names(code);
        if !new_fns.is_empty() && new_fns.iter().all(|f| has_function(&existing, f)) {
            return Ok(()); // All functions already exist
        }
        let merged = deduplicate_imports(&format!("{}\n\n{}\n", existing, code));
        fs::write(&file_path, merged).map_err(|e| e.to_string())?;
    } else {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&file_path, code).map_err(|e| e.to_string())?;
        register_module(project_dir, target_file)?;
    }
    Ok(())
}

/// Add `pub mod <name>;` to the nearest lib.rs or mod.rs for a new file.
pub fn register_module(project_dir: &Path, file_path: &str) -> Result<(), String> {
    let mod_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file path")?;

    // Find the parent lib.rs or mod.rs
    let full_path = project_dir.join(file_path);
    let parent_dir = full_path.parent().ok_or("No parent directory")?;
    let lib_rs = parent_dir.join("lib.rs");
    let mod_rs = parent_dir.join("mod.rs");

    let registry_file = if lib_rs.exists() {
        lib_rs
    } else if mod_rs.exists() {
        mod_rs
    } else {
        return Ok(()); // No parent module file — skip
    };

    let content = fs::read_to_string(&registry_file).map_err(|e| e.to_string())?;
    let mod_line = format!("pub mod {};", mod_name);
    if !content.contains(&mod_line) {
        let updated = format!("{}\n{}\n", content.trim_end(), mod_line);
        fs::write(&registry_file, updated).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Check if a function with the given name exists in the content.
pub fn has_function(content: &str, fn_name: &str) -> bool {
    let pattern = format!("fn {}(", fn_name);
    content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.contains(&pattern)
    })
}

/// Extract function names from Rust code.
fn extract_fn_names(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                let after_fn = trimmed.split("fn ").nth(1)?;
                let name = after_fn.split('(').next()?.trim();
                if !name.is_empty() { Some(name.to_string()) } else { None }
            } else {
                None
            }
        })
        .collect()
}

/// Remove duplicate `use` import lines.
pub fn deduplicate_imports(content: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") && trimmed.ends_with(';') {
            if !seen.insert(trimmed.to_string()) {
                continue; // Skip duplicate import
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_function() {
        let code = "pub fn hello() -> String {\n    \"hi\".into()\n}\n";
        assert!(has_function(code, "hello"));
        assert!(!has_function(code, "goodbye"));
    }

    #[test]
    fn test_extract_fn_names() {
        let code = "pub fn foo() {}\nfn bar(x: i32) {}\n";
        let names = extract_fn_names(code);
        assert_eq!(names, vec!["foo", "bar"]);
    }

    #[test]
    fn test_deduplicate_imports() {
        let code = "use std::fs;\nuse std::path::Path;\nuse std::fs;\n\nfn main() {}\n";
        let result = deduplicate_imports(code);
        assert_eq!(result.matches("use std::fs;").count(), 1);
        assert!(result.contains("use std::path::Path;"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_has_function_with_pub() {
        let code = "    pub fn my_func(a: u32) -> bool { true }\n";
        assert!(has_function(code, "my_func"));
    }
}
