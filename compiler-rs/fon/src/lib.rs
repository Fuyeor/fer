// compiler-rs/fon/src/lib.rs

use fon_parser::{Diagnostic as FonDiagnostic, ParseResult};
use infra::{Diagnostic, DiagnosticBag, Severity, Span};
use vfs::{FileId, SourceMap};

pub mod query;

pub use query::{FON_PARSE_QUERY, FON_SOURCE_QUERY, register_queries, set_source_file};

/// A parsed FON source together with the Fer file identity that owns it.
#[derive(Debug, Clone)]
pub struct ParsedFonSource {
    pub file_id: FileId,
    pub result: ParseResult,
}

/// Parse one in-memory FON source without accessing Fer compiler state.
pub fn parse_source(file_id: FileId, source: &str) -> ParsedFonSource {
    ParsedFonSource {
        file_id,
        result: fon_parser::parse(source),
    }
}

/// Parse a source file already owned by Fer's virtual source map.
pub fn parse_file(source_map: &SourceMap, file_id: FileId) -> Option<ParsedFonSource> {
    source_map
        .content(file_id)
        .map(|source| parse_source(file_id, source))
}

/// Resolve parsed FON values through a caller-provided Fer scheme.
pub fn resolve_source(
    parsed: &ParsedFonSource,
    resolver: &dyn fon_parser::SchemeResolver,
) -> fon_parser::ResolveResult {
    fon_parser::resolve(&parsed.result.document, resolver)
}

/// Convert FON parser diagnostics into Fer's diagnostic collector.
pub fn report_diagnostics(parsed: &ParsedFonSource, diagnostics: &mut DiagnosticBag) {
    for diagnostic in &parsed.result.diagnostics {
        diagnostics.add(to_fer_diagnostic(parsed.file_id, diagnostic));
    }
}

fn to_fer_diagnostic(file_id: FileId, diagnostic: &FonDiagnostic) -> Diagnostic {
    let _ = file_id;
    Diagnostic::new(
        Severity::Error,
        map_code(diagnostic.code.as_str()),
        format_message(diagnostic),
        Span::new(diagnostic.span.start as usize, diagnostic.span.end as usize),
    )
}

/// Convert FON resolution diagnostics into Fer's diagnostic collector.
pub fn report_resolution_diagnostics(
    resolved: &fon_parser::ResolveResult,
    diagnostics: &mut DiagnosticBag,
) {
    for diagnostic in &resolved.diagnostics {
        diagnostics.add(Diagnostic::new(
            Severity::Error,
            map_code(diagnostic.code.as_str()),
            format_message(diagnostic),
            Span::new(diagnostic.span.start as usize, diagnostic.span.end as usize),
        ));
    }
}

fn map_code(code: &str) -> &'static str {
    match code {
        "E0001" => "fon-max-depth",
        "E0002" => "fon-max-tokens",
        "E0003" => "fon-invalid-utf8",
        "E0004" => "fon-max-token-length",
        "E0101" => "fon-expected-root",
        "E0102" => "fon-expected-key",
        "E0103" => "fon-expected-closing-brace",
        "E0104" => "fon-expected-equals",
        "E0105" => "fon-expected-member",
        "E0106" => "fon-expected-value",
        "E0107" => "fon-expected-closing-bracket",
        "E0110" => "fon-expected-annotation-equals",
        "E0111" => "fon-expected-annotation-argument",
        "E0112" => "fon-expected-closing-annotation",
        "E0201" => "fon-expected-type",
        "E0202" => "fon-expected-type-argument",
        "E0203" => "fon-expected-type-closer",
        "E0301" => "fon-expected-schema-member",
        "E0401" => "fon-expected-schema",
        "E0402" => "fon-expected-schema-type",
        "E0403" => "fon-expected-schema-body",
        "E0404" => "fon-expected-schema-field",
        "E0405" => "fon-expected-enum-variant",
        "E0406" => "fon-expected-enum-payload",
        "E1001" => "fon-duplicate-key",
        "E1002" => "fon-scheme-resolution",
        "E1003" => "fon-invalid-type",
        _ => "fon-parse-error",
    }
}

fn format_message(diagnostic: &FonDiagnostic) -> String {
    format!("[{}] {}", diagnostic.code, diagnostic.message)
}
