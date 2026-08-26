// compiler-rs/diagnostics/tests/catalog_tests.rs

use diagnostics::{Catalog, CatalogError, Locale, RenderError};
use infra::{Diagnostic, DiagnosticValue, MessageId, Span};

const CATALOG_SOURCE: &str = r#"
analysis = {
  type-mismatch = {
    en = `expected {expected}, found {found}`
    zh-hans = `期望类型为 {expected}，但实际找到了 {found}`
    desc = {
      en = `this expression has type {found}`
      zh-hans = `此表达式的类型为 {found}`
    }
  }
}
"#;

fn type_mismatch() -> Diagnostic {
    Diagnostic::error(
        "type-mismatch",
        MessageId::new("analysis.type-mismatch"),
        Span::new(0, 4),
    )
    .with_arg("expected", DiagnosticValue::Type(String::from("int")))
    .with_arg("found", DiagnosticValue::Type(String::from("string")))
}

#[test]
fn embedded_catalog_is_valid() {
    let catalog = Catalog::embedded().expect("embedded catalog should load");
    assert!(catalog.contains(MessageId::new("analysis.type-mismatch")));
}

#[test]
fn nested_objects_flatten_to_message_ids() {
    let catalog = Catalog::from_source(CATALOG_SOURCE).expect("catalog should load");

    assert!(catalog.contains(MessageId::new("analysis.type-mismatch")));
    assert!(catalog.contains(MessageId::new("analysis.type-mismatch.desc")));
}

#[test]
fn renderer_uses_requested_locale() {
    let catalog = Catalog::from_source(CATALOG_SOURCE).expect("catalog should load");
    let rendered = catalog
        .render(&type_mismatch(), Locale::new("zh-hans"))
        .expect("diagnostic should render");

    assert_eq!(rendered.message, "期望类型为 int，但实际找到了 string");
}

#[test]
fn renderer_falls_back_to_english_for_unknown_locale() {
    let catalog = Catalog::from_source(CATALOG_SOURCE).expect("catalog should load");
    let rendered = catalog
        .render(&type_mismatch(), Locale::new("fr"))
        .expect("english fallback should render");

    assert_eq!(rendered.message, "expected int, found string");
}

#[test]
fn missing_render_argument_is_reported_without_panicking() {
    let catalog = Catalog::from_source(CATALOG_SOURCE).expect("catalog should load");
    let diagnostic = Diagnostic::error(
        "type-mismatch",
        MessageId::new("analysis.type-mismatch"),
        Span::new(0, 4),
    )
    .with_arg("expected", DiagnosticValue::Type(String::from("int")));

    let error = catalog
        .render(&diagnostic, Locale::new("en"))
        .expect_err("missing placeholder should be rejected");
    assert!(matches!(error, RenderError::MissingArgument { .. }));
}

#[test]
fn missing_english_translation_is_a_catalog_error() {
    let source = r#"
analysis = {
  unknown-type = {
    zh-hans = `无法找到类型 {name}`
  }
}
"#;

    let error = Catalog::from_source(source).expect_err("en is required");
    assert!(matches!(error, CatalogError::MissingDefaultLocale { .. }));
}

#[test]
fn non_string_translation_is_a_catalog_error() {
    let source = r#"
analysis = {
  invalid = {
    en = 1
  }
}
"#;

    let error = Catalog::from_source(source).expect_err("translation must be a string");
    assert!(matches!(error, CatalogError::InvalidTranslation { .. }));
}

#[test]
fn invalid_placeholder_syntax_is_a_catalog_error() {
    let source = r#"
analysis = {
  invalid = {
    en = `expected {expected + found}`
  }
}
"#;

    let error = Catalog::from_source(source).expect_err("placeholder must be one identifier");
    assert!(matches!(error, CatalogError::InvalidPlaceholder { .. }));
}
