// infra/src/diag/mod.rs
/// Fer Diagnostic System
///
/// This module provides the types and collectors for compiler diagnostics.
/// Rendering is intentionally kept out of infra/; it belongs to a higher layer
/// (e.g. a future `driver/` or `cli/`) that has access to source files.
pub mod diagnostic;
pub mod diagnostic_bag;

// Re-export key types so users can just `use infra::diag::*`
pub use diagnostic::{Diagnostic, Severity};
pub use diagnostic_bag::DiagnosticBag;
