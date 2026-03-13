//! Filesystem watcher with debouncing for proactive suggestions.
//!
//! Uses `notify` to watch a project directory and emits debounced `FileChange`
//! events via a crossbeam channel.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::debug;

/// A filesystem change event (debounced).
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub timestamp: Instant,
}

/// The kind of filesystem change detected.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Filters out paths we never want to watch.
fn should_ignore(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy();
    // Directory-based ignores
    if path_str.contains("/.git/")
        || path_str.contains("/target/")
        || path_str.contains("/node_modules/")
    {
        return true;
    }
    // File-based ignores
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name == ".DS_Store"
            || name.ends_with(".swp")
            || name.ends_with(".swo")
            || name.ends_with('~')
        {
            return true;
        }
    }
    false
}

/// Map a `notify` event kind to our simplified `ChangeKind`.
fn map_event_kind(kind: &EventKind) -> Option<ChangeKind> {
    match kind {
        EventKind::Create(_) => Some(ChangeKind::Created),
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => Some(ChangeKind::Renamed),
        EventKind::Modify(_) => Some(ChangeKind::Modified),
        EventKind::Remove(_) => Some(ChangeKind::Deleted),
        _ => None,
    }
}

/// Watches a project directory for file changes with debouncing.
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    receiver: Receiver<FileChange>,
    root: PathBuf,
    _raw_receiver: Receiver<Event>,
}

/// Debounce window — events on the same path within this window are merged.
const DEBOUNCE_MS: u128 = 500;

impl FileWatcher {
    /// Start watching a directory. Spawns a background OS thread for debouncing.
    pub fn start(root: PathBuf) -> Result<Self, String> {
        let (raw_tx, raw_rx) = crossbeam_channel::unbounded::<Event>();
        let (change_tx, change_rx) = crossbeam_channel::unbounded::<FileChange>();

        // Create the OS watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = raw_tx.send(event);
            }
        })
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch {}: {e}", root.display()))?;

        // Spawn debounce thread (OS thread, not tokio)
        let raw_rx_clone = raw_rx.clone();
        std::thread::Builder::new()
            .name("hydra-file-debounce".into())
            .spawn(move || {
                Self::debounce_loop(raw_rx_clone, change_tx);
            })
            .map_err(|e| format!("Failed to spawn debounce thread: {e}"))?;

        debug!("FileWatcher started on {}", root.display());

        Ok(Self {
            watcher: Some(watcher),
            receiver: change_rx,
            root,
            _raw_receiver: raw_rx,
        })
    }

    /// Debounce loop: runs on a dedicated OS thread.
    fn debounce_loop(raw_rx: Receiver<Event>, change_tx: Sender<FileChange>) {
        let mut pending: HashMap<PathBuf, (ChangeKind, Instant)> = HashMap::new();

        loop {
            // Wait up to 200ms for the next raw event
            match raw_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(event) => {
                    if let Some(change_kind) = map_event_kind(&event.kind) {
                        for path in &event.paths {
                            if !should_ignore(path) {
                                pending.insert(path.clone(), (change_kind.clone(), Instant::now()));
                            }
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    debug!("FileWatcher raw channel disconnected, stopping debounce");
                    break;
                }
            }

            // Flush entries older than the debounce window
            let now = Instant::now();
            let ready: Vec<_> = pending
                .iter()
                .filter(|(_, (_, ts))| now.duration_since(*ts).as_millis() >= DEBOUNCE_MS)
                .map(|(p, (k, ts))| (p.clone(), k.clone(), *ts))
                .collect();

            for (path, kind, timestamp) in ready {
                pending.remove(&path);
                if change_tx
                    .send(FileChange {
                        path,
                        kind,
                        timestamp,
                    })
                    .is_err()
                {
                    return; // receiver dropped
                }
            }
        }
    }

    /// Drain all pending changes (non-blocking).
    pub fn drain_changes(&self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(change) => changes.push(change),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        changes
    }

    /// The root directory being watched.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Stop watching. Drops the OS watcher handle.
    pub fn stop(&mut self) {
        if let Some(mut w) = self.watcher.take() {
            let _ = w.unwatch(&self.root);
            debug!("FileWatcher stopped for {}", self.root.display());
        }
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore() {
        assert!(should_ignore(std::path::Path::new("/project/.git/config")));
        assert!(should_ignore(std::path::Path::new("/project/target/debug/foo")));
        assert!(should_ignore(std::path::Path::new("/project/.DS_Store")));
        assert!(should_ignore(std::path::Path::new("/project/foo.swp")));
        assert!(!should_ignore(std::path::Path::new("/project/src/main.rs")));
        assert!(!should_ignore(std::path::Path::new("/project/Cargo.toml")));
    }

    #[test]
    fn test_map_event_kind() {
        use notify::event::*;
        assert_eq!(
            map_event_kind(&EventKind::Create(CreateKind::File)),
            Some(ChangeKind::Created)
        );
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Data(DataChange::Content))),
            Some(ChangeKind::Modified)
        );
        assert_eq!(
            map_event_kind(&EventKind::Remove(RemoveKind::File)),
            Some(ChangeKind::Deleted)
        );
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Name(RenameMode::Both))),
            Some(ChangeKind::Renamed)
        );
        assert_eq!(map_event_kind(&EventKind::Other), None);
    }

    #[test]
    fn test_change_kind_equality() {
        assert_eq!(ChangeKind::Created, ChangeKind::Created);
        assert_ne!(ChangeKind::Created, ChangeKind::Modified);
    }
}
