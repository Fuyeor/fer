// infra/src/diag/diagnostic_bag.rs
/// A collector for diagnostics that allows ordering and filtering.
/// The bag owns all diagnostics emitted during a compilation phase.
use super::diagnostic::{Diagnostic, Severity};
use std::collections::BTreeMap;

/// Accumulates diagnostics and provides query methods.
/// The bag is the single point where diagnostics are collected;
/// rendering is done elsewhere (e.g. a `DiagnosticRenderer` in a later layer).
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    /// Stored in insertion order but also indexed by severity for fast filtering.
    /// We use a Vec as primary storage (preserves order) and maintain a
    /// severity-to-count summary for quick checks.
    diagnostics: Vec<Diagnostic>,
    count_by_severity: BTreeMap<Severity, usize>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            count_by_severity: BTreeMap::new(),
        }
    }

    /// Add a diagnostic to the bag.
    pub fn add(&mut self, diagnostic: Diagnostic) {
        *self
            .count_by_severity
            .entry(diagnostic.severity)
            .or_insert(0) += 1;
        self.diagnostics.push(diagnostic);
    }

    /// Returns true if there is at least one error in the bag.
    pub fn has_errors(&self) -> bool {
        self.count_by_severity
            .get(&Severity::Error)
            .copied()
            .unwrap_or(0)
            > 0
    }

    /// Returns the number of diagnostics of a given severity.
    pub fn count(&self, severity: Severity) -> usize {
        self.count_by_severity.get(&severity).copied().unwrap_or(0)
    }

    /// Iterate over all diagnostics in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Consumes the bag and returns the diagnostics, e.g. for rendering.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}
