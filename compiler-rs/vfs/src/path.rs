// vfs/src/path.rs

use infra::symbol::{Interner, Symbol};

/// A canonical, interned virtual path.
///
/// `VirtualPath` is `Copy` and comparison is an integer operation.
/// It enforces Fer’s naming rules:
/// - All directory and file names must be kebab‑case (`[a-z0-9]+(-[a-z0-9]+)*`).
/// - File extension must be `.fer`.
/// - Parent traversal (`..`) and dot segments (`.`) are forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualPath(Symbol);

impl VirtualPath {
    /// Attempt to create a `VirtualPath` from a raw string.
    ///
    /// Returns `None` if the path contains illegal characters or segments.
    /// On success, the path string is interned via `interner`.
    pub fn new(raw: &str, interner: &mut Interner) -> Option<Self> {
        let normalized = raw.replace('\\', "/");

        // Reject parent traversal and dot segments
        if normalized.contains("..") || normalized.contains("./") || normalized.starts_with('.') {
            return None;
        }
        // Reject empty segments
        if normalized.split('/').any(|s| s.is_empty()) {
            return None;
        }

        // Validate each component (directories + final file name)
        for component in normalized.split('/') {
            if !Self::is_valid_component(component) {
                return None;
            }
        }

        // Ensure the last component (file name) ends with `.fer`
        if !normalized.ends_with(".fer") {
            return None;
        }

        let sym = interner.intern(&normalized);
        Some(Self(sym))
    }

    /// Returns the interned string of this path.
    pub fn as_str<'a>(&self, interner: &'a Interner) -> &'a str {
        interner.lookup(self.0).unwrap_or("<invalid-path>")
    }

    /// Internal: validate that a single path component is kebab‑case.
    /// File names include the `.fer` extension; we strip it before checking.
    fn is_valid_component(component: &str) -> bool {
        let name = component.strip_suffix(".fer").unwrap_or(component);
        if name.is_empty() {
            return false; // e.g. ".fer" alone is invalid
        }
        name.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !name.starts_with('-')
            && !name.ends_with('-')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_paths() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("src/main.fer", &mut interner).is_some());
        assert!(VirtualPath::new("my-package/utils.fer", &mut interner).is_some());
        assert!(VirtualPath::new("src\\main.fer", &mut interner).is_some()); // normalized
    }

    #[test]
    fn rejects_parent_traversal() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("../secret.fer", &mut interner).is_none());
        assert!(VirtualPath::new("a/../b.fer", &mut interner).is_none());
    }

    #[test]
    fn rejects_dot_segments() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("./main.fer", &mut interner).is_none());
        assert!(VirtualPath::new("a/./b.fer", &mut interner).is_none());
    }

    #[test]
    fn rejects_empty_components() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("a//b.fer", &mut interner).is_none());
        assert!(VirtualPath::new("/absolute.fer", &mut interner).is_none()); // leading slash is empty first component
    }

    #[test]
    fn rejects_uppercase_in_name() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("src/Main.fer", &mut interner).is_none());
        assert!(VirtualPath::new("src/utils/UserRepo.fer", &mut interner).is_none());
    }

    #[test]
    fn rejects_underscore_in_name() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("src/my_util.fer", &mut interner).is_none());
    }

    #[test]
    fn rejects_missing_fer_extension() {
        let mut interner = Interner::new();
        assert!(VirtualPath::new("src/main.txt", &mut interner).is_none());
        assert!(VirtualPath::new("src/main", &mut interner).is_none());
    }

    #[test]
    fn interned_paths_are_equal() {
        let mut interner = Interner::new();
        let a = VirtualPath::new("src/lib.fer", &mut interner).unwrap();
        let b = VirtualPath::new("src/lib.fer", &mut interner).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.as_str(&interner), "src/lib.fer");
    }
}
