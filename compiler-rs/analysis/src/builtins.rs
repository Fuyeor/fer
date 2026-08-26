// compiler-rs/analysis/src/builtins.rs

/// A language-level builtin recognized without a user-defined HIR item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinKind {
    Print,
}

impl BuiltinKind {
    /// Resolve a builtin from its source-level name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "print" => Some(Self::Print),
            _ => None,
        }
    }
}
