// syntax/src/lex/string_tests.rs

use infra::Interner;

use super::{Lexer, TokenKind};

#[test]
fn interpolated_string_tokens() {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new("`Hello, {name}!`", &mut interner);
    let mut kinds = Vec::new();
    loop {
        let token = lexer.next_token();
        kinds.push(token.kind);
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    assert_eq!(
        kinds,
        [
            TokenKind::StringStart,
            TokenKind::StringPart,
            TokenKind::ExprStart,
            TokenKind::Identifier,
            TokenKind::ExprEnd,
            TokenKind::StringPart,
            TokenKind::StringEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn multiline_string_dedents_and_joins_physical_lines() {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new("`\n  one\n  two\n  `", &mut interner);
    let token = lexer.next_token();
    assert_eq!(token.kind, TokenKind::StringLiteral);
    assert_eq!(
        token.symbol.and_then(|symbol| interner.lookup(symbol)),
        Some("one\ntwo")
    );

    let mut continued = Lexer::new("`one \\\n  two`", &mut interner);
    let token = continued.next_token();
    assert_eq!(
        token.symbol.and_then(|symbol| interner.lookup(symbol)),
        Some("one two")
    );
}

#[test]
fn escaped_braces_remain_plain_string_text() {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(r#"`\{name\}`"#, &mut interner);
    let token = lexer.next_token();

    assert_eq!(token.kind, TokenKind::StringLiteral);
    assert_eq!(
        token.symbol.and_then(|symbol| interner.lookup(symbol)),
        Some("{name}")
    );
}
