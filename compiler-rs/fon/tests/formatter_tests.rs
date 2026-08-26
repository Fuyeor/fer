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
fn preserves_native_atoms_and_mixed_separators() {
    let source = "package={name=@fer/std,version=0.1.0\nlicense=.mit,path=./docs/index.md,enabled=true,constraint=^0.1.0}\n";
    let formatted = format_source(source).expect("native FON atoms must format");

    assert!(
        formatted.contains("name = @fer/std"),
        "formatted: {formatted}"
    );
    assert!(
        formatted.contains("version = 0.1.0"),
        "formatted: {formatted}"
    );
    assert!(
        formatted.contains("license = .mit"),
        "formatted: {formatted}"
    );
    assert!(
        formatted.contains("path = ./docs/index.md"),
        "formatted: {formatted}"
    );
    assert!(
        formatted.contains("enabled = true"),
        "formatted: {formatted}"
    );
    assert!(
        formatted.contains("constraint = ^0.1.0"),
        "formatted: {formatted}"
    );
    assert!(
        !formatted.contains("`@fer/std`"),
        "native package path became a string"
    );
    assert!(
        !formatted.contains("`.mit`"),
        "native enum atom became a string"
    );
    assert!(
        !formatted.contains("`0.1.0`"),
        "native version became a string"
    );
    assert!(
        !formatted.contains("`./docs/index.md`"),
        "native relative path became a string"
    );
    assert_eq!(
        format_source(&formatted).expect("native FON output must be idempotent"),
        formatted
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
