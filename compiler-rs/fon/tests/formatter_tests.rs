// compiler-rs/fon/tests/formatter_tests.rs

use fon::{FormatError, format_source};

#[test]
fn formats_fon_spacing_and_nested_object_indentation_losslessly() {
    let source = "// leading\nname=`Fuyeor`\nconfig={\nmessage=`a+b={x}` // keep  +  \npattern=/a + b/i\n}\n";
    let formatted = format_source(source).expect("valid FON source must format");

    assert_eq!(
        formatted,
        "// leading\nname = `Fuyeor`\nconfig = {\n  message = `a+b={x}` // keep  +  \n  pattern = /a + b/i\n}\n"
    );
    assert_eq!(
        format_source(&formatted).expect("formatted FON source must be idempotent"),
        formatted
    );
}

#[test]
fn preserves_crlf_and_comment_separated_gaps() {
    let source = "config={\r\nvalue=1/* keep */\r\nnext=2\r\n}\r\n";
    let formatted = format_source(source).expect("valid FON source must format");

    assert_eq!(
        formatted,
        "config = {\r\n  value = 1/* keep */\r\n  next = 2\r\n}\r\n"
    );
}

#[test]
fn rejects_invalid_fon_without_returning_a_rewrite() {
    assert!(matches!(
        format_source("config={\nvalue=\n"),
        Err(FormatError::Parse { .. })
    ));
}

#[test]
fn formats_the_repository_locale_losslessly_and_idempotently() {
    let source = include_str!("../../../locale/locales.fon");
    let formatted = format_source(source).expect("repository locale must format");

    assert!(formatted.starts_with("// locale/locales.fon\n"));
    assert_eq!(
        format_source(&formatted).expect("formatted locale must be idempotent"),
        formatted
    );
}
