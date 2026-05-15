// vfs/src/overlay.rs

use crate::source::FileId;
use std::collections::HashMap;

/// Virtual File System with Overlay support for time-safe migrations.
pub struct Vfs {
    physical_files: HashMap<FileId, String>,
    overlays: HashMap<FileId, String>,
}

impl Vfs {
    pub fn read(&self, id: FileId) -> &str {
        self.overlays
            .get(&id)
            .map(|s| s.as_str())
            .unwrap_or_else(|| {
                self.physical_files
                    .get(&id)
                    .map(|s| s.as_str())
                    .unwrap_or("")
            })
    }
}
