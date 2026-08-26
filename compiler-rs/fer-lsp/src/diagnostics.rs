// compiler-rs/fer-lsp/src/diagnostics.rs

use diagnostics::{Catalog, Locale};
use infra::{Diagnostic as CoreDiagnostic, Severity};
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity, NumberOrString};

use crate::position::{LineIndex, to_lsp_range};

/// Convert all structured compiler diagnostics into LSP diagnostics.
pub fn to_lsp_diagnostics(
    diagnostics: &[CoreDiagnostic],
    line_index: &LineIndex,
) -> Result<Vec<Diagnostic>, String> {
    let catalog = Catalog::embedded()
        .map_err(|error| format!("failed to load diagnostics catalog: {error:?}"))?;
    diagnostics
        .iter()
        .map(|diagnostic| to_lsp_diagnostic(diagnostic, line_index, &catalog))
        .collect()
}

/// Render and convert one compiler diagnostic at the protocol boundary.
fn to_lsp_diagnostic(
    diagnostic: &CoreDiagnostic,
    line_index: &LineIndex,
    catalog: &Catalog,
) -> Result<Diagnostic, String> {
    let rendered = catalog
        .render(diagnostic, Locale::new("en"))
        .map_err(|error| format!("failed to render diagnostic {}: {error:?}", diagnostic.code))?;
    let message = append_notes(rendered.message, &rendered.notes);
    Ok(Diagnostic::new(
        to_lsp_range(line_index.range(rendered.primary)),
        Some(to_lsp_severity(rendered.severity)),
        Some(NumberOrString::String(rendered.code.to_owned())),
        Some("fer".to_owned()),
        message,
        None,
        None,
    ))
}

/// Append related notes without inventing a second protocol diagnostic.
fn append_notes(mut message: String, notes: &[String]) -> String {
    for note in notes {
        message.push_str("\n\nnote: ");
        message.push_str(note);
    }
    message
}

/// Map compiler severity to the corresponding LSP severity.
const fn to_lsp_severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::INFORMATION,
    }
}
