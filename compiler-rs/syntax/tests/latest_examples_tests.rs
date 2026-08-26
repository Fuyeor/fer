// compiler-rs/syntax/tests/latest_examples_tests.rs

use syntax::{format_source, parse_lossless_cst};
use vfs::FileId;

const EXAMPLES: &[(&str, &str)] = &[
    (
        "condition.fer",
        include_str!("../../../examples/condition.fer"),
    ),
    (
        "constants.fer",
        include_str!("../../../examples/constants.fer"),
    ),
    ("example.fer", include_str!("../../../examples/example.fer")),
    (
        "function.fer",
        include_str!("../../../examples/function.fer"),
    ),
];

#[test]
fn d11_examples_parse_as_current_fer_syntax() {
    for (name, source) in EXAMPLES {
        let snapshot = parse_lossless_cst(source, FileId(0))
            .unwrap_or_else(|error| panic!("{name} must parse: {error:?}"));
        assert!(snapshot.root().0 > 0, "{name} must have a module root");
    }
}

#[test]
fn headerless_source_does_not_receive_a_synthetic_path_comment() {
    let source = "answer=40+2\n";
    let formatted = format_source(source).expect("headerless source must format");

    assert_eq!(formatted, "answer = 40 + 2\n");
    assert!(!formatted.starts_with("/// @/"));
}

#[test]
fn d11_examples_format_losslessly_and_idempotently() {
    for (name, source) in EXAMPLES {
        let formatted =
            format_source(source).unwrap_or_else(|error| panic!("{name} must format: {error:?}"));
        let reformatted = format_source(&formatted)
            .unwrap_or_else(|error| panic!("{name} must format twice: {error:?}"));
        assert_eq!(
            formatted, reformatted,
            "{name} formatter must be idempotent"
        );
        assert!(
            formatted.starts_with("/// @/examples/"),
            "{name} must preserve its source-location header"
        );
    }
}
