// compiler-rs/fon/tests/adapter_tests.rs

use fon::{parse_file, parse_source, report_diagnostics, resolve_source};
use infra::{DiagnosticBag, Severity, Span};
use vfs::{FileId, SourceMap};

struct TestScheme;

impl fon_parser::SchemeResolver for TestScheme {
    fn resolve_type(
        &self,
        name: &str,
    ) -> Result<fon_parser::TypeReference, fon_parser::SchemeError> {
        Ok(fon_parser::TypeReference {
            name: name.into(),
            span: None,
        })
    }

    fn resolve_unknown(
        &self,
        expected_type: Option<&fon_parser::TypeReference>,
        value: &fon_parser::UnknownValue,
    ) -> Result<fon_parser::TypedAtom, fon_parser::SchemeError> {
        Ok(fon_parser::TypedAtom {
            type_name: expected_type
                .map(|type_reference| type_reference.name.clone())
                .unwrap_or_else(|| "Unknown".into()),
            raw: value.raw.clone(),
            span: value.span,
        })
    }
}

#[test]
fn parses_fon_source_without_copying_fer_state_into_the_parser() {
    let parsed = parse_source(FileId(7), "name = `Fuyeor`\n");

    assert_eq!(parsed.file_id, FileId(7));
    assert!(!parsed.result.has_errors());
    assert_eq!(parsed.result.document.source(), "name = `Fuyeor`\n");
}

#[test]
fn parses_a_file_from_fer_source_map() {
    let mut source_map = SourceMap::new();
    let file_id = source_map
        .add_file("config.fer", "name = `Fuyeor`\n".into())
        .expect("valid FON path");

    let parsed = parse_file(&source_map, file_id).expect("source file");

    assert_eq!(parsed.file_id, file_id);
    assert!(!parsed.result.has_errors());
}

#[test]
fn resolves_fon_values_through_a_fer_scheme() {
    let parsed = parse_source(FileId(4), "color: Hex = #AEA4E4\n");
    let resolved = resolve_source(&parsed, &TestScheme);

    assert!(!resolved.has_errors());
    let fon_parser::TypedRoot::Object(members) = resolved.document.root else {
        panic!("expected object root");
    };
    assert!(matches!(members[0].value, fon_parser::TypedValue::Atom(_)));
}

#[test]
fn maps_fon_diagnostics_to_fer_diagnostic_bag() {
    let parsed = parse_source(FileId(3), "name =\n");
    let mut bag = DiagnosticBag::new();

    report_diagnostics(&parsed, &mut bag);

    assert!(bag.has_errors());
    let diagnostic = bag.iter().next().expect("mapped diagnostic");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.span, Span::new(6, 7));
}
