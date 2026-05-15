// vfs/src/watcher.rs

/// Placeholder for file system monitoring.
/// Will be used to invalidate incremental compilation queries when source files change.
#[derive(Debug, Default)]
pub struct FileWatcher {
    // Will hold platform‑specific watcher state (e.g., notify::Watcher).
}

impl FileWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any watched files have changed. Always returns `false` for now.
    pub fn has_changes(&self) -> bool {
        false
    }
}
