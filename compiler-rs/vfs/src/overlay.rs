// vfs/src/overlay.rs

use crate::source::{FileId, SourceMap};
use std::collections::HashMap;

pub struct Vfs {
    source_map: SourceMap,
    overlays: HashMap<FileId, String>,
}

impl Vfs {
    pub fn new(source_map: SourceMap) -> Self {
        Self {
            source_map,
            overlays: HashMap::new(),
        }
    }

    pub fn set_overlay(&mut self, id: FileId, content: String) {
        self.overlays.insert(id, content);
    }

    pub fn clear_overlay(&mut self, id: FileId) {
        self.overlays.remove(&id);
    }

    pub fn read(&self, id: FileId) -> &str {
        if let Some(overlay) = self.overlays.get(&id) {
            overlay.as_str()
        } else {
            self.source_map.content(id).unwrap_or("")
        }
    }

    pub fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Add a physical file. Returns `None` if the path is invalid.
    pub fn add_physical_file(&mut self, path: &str, content: String) -> Option<FileId> {
        self.source_map.add_file(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_overrides_physical() {
        let mut vfs = Vfs::new(SourceMap::new());
        let id = vfs
            .add_physical_file("main.fer", "original".into())
            .unwrap();
        vfs.set_overlay(id, "overlaid".into());
        assert_eq!(vfs.read(id), "overlaid");
    }

    #[test]
    fn clear_overlay_restores_original() {
        let mut vfs = Vfs::new(SourceMap::new());
        let id = vfs
            .add_physical_file("main.fer", "original".into())
            .unwrap();
        vfs.set_overlay(id, "overlaid".into());
        vfs.clear_overlay(id);
        assert_eq!(vfs.read(id), "original");
    }

    #[test]
    fn read_nonexistent_file_returns_empty() {
        let vfs = Vfs::new(SourceMap::new());
        assert_eq!(vfs.read(FileId(999)), "");
    }
}
