// infra/src/lib.rs

pub mod diag;
pub mod profile;
pub mod span;
pub mod symbol;

// Re-export commonly used types at the infra level for convenience.
// This is intentional: infra is the foundation that all other crates depend on.
pub use diag::{Diagnostic, DiagnosticBag, Severity};
pub use span::Span;
pub use symbol::{Interner, Symbol};
