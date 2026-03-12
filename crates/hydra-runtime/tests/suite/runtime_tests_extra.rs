use hydra_runtime::filesystem::{init_filesystem, verify_filesystem};

// ═══════════════════════════════════════════════════════════
// FILESYSTEM PERMISSION TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn test_filesystem_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("hydra-data");
    init_filesystem(&data_dir).unwrap();

    let mode = std::fs::metadata(&data_dir).unwrap().permissions().mode();
    // Check owner rwx (0o700)
    assert_eq!(mode & 0o777, 0o700);

    let receipts_mode = std::fs::metadata(data_dir.join("receipts"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(receipts_mode & 0o777, 0o700);
}
