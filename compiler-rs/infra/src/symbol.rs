// infra/src/symbol.rs

/// A unique identifier for interning strings.
/// This ensures that comparing two identifiers is a simple O(1) integer comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

impl Symbol {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}
