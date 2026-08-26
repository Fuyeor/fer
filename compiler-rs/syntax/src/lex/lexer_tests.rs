// syntax/src/lex/lexer_tests.rs

use super::*;
use crate::grammar::TokenKind;

fn lex_one(source: &str) -> Token {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new(source, &mut interner);
    lexer.next_token()
}

#[test]
fn eof_token() {
    let tok = lex_one("");
    assert_eq!(tok.kind, TokenKind::Eof);
}

#[test]
fn integer_literal() {
    let tok = lex_one("42");
    assert_eq!(tok.kind, TokenKind::IntLiteral);
    assert_eq!(tok.span, Span::new(0, 2));
}

#[test]
fn float_literal() {
    let tok = lex_one("3.14");
    assert_eq!(tok.kind, TokenKind::FloatLiteral);
}

#[test]
fn identifier_kebab_case() {
    let tok = lex_one("my-var");
    assert_eq!(tok.kind, TokenKind::Identifier);
    assert!(tok.symbol.is_some());
}

#[test]
fn identifier_with_underscore_allowed() {
    let tok = lex_one("my_var"); // will be checked in semantic phase
    assert_eq!(tok.kind, TokenKind::Identifier);
}

#[test]
fn identifier_capital_allowed() {
    let tok = lex_one("StructName"); // valid for struct/enum names
    assert_eq!(tok.kind, TokenKind::Identifier);
}

#[test]
fn keyword_enum() {
    let tok = lex_one("enum");
    assert_eq!(tok.kind, TokenKind::Enum);
    assert!(tok.symbol.is_none());
}

#[test]
fn logical_words_are_contextual_identifiers() {
    for word in ["and", "or", "all", "any", "one", "none"] {
        assert_eq!(
            lex_one(word).kind,
            TokenKind::Identifier,
            "{word} must remain contextual"
        );
    }
}

#[test]
fn simple_string() {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new("`hello`", &mut interner);
    let tok = lexer.next_token();
    assert_eq!(tok.kind, TokenKind::StringLiteral);
    if let Some(sym) = tok.symbol {
        assert_eq!(interner.lookup(sym), Some("hello"));
    } else {
        panic!("Expected symbol");
    }
}

#[test]
fn operators() {
    assert_eq!(lex_one("+").kind, TokenKind::Plus);
    assert_eq!(lex_one("-").kind, TokenKind::Minus);
    assert_eq!(lex_one("*").kind, TokenKind::Star);
    assert_eq!(lex_one("/").kind, TokenKind::Slash);
    assert_eq!(lex_one("<").kind, TokenKind::Lt);
    assert_eq!(lex_one(">").kind, TokenKind::Gt);
    assert_eq!(lex_one("<=").kind, TokenKind::LtEq);
    assert_eq!(lex_one(">=").kind, TokenKind::GtEq);
    assert_eq!(lex_one("=").kind, TokenKind::Eq);
    assert_eq!(lex_one("->").kind, TokenKind::Arrow);
}

#[test]
fn at_symbol() {
    assert_eq!(lex_one("@").kind, TokenKind::At);
}

#[test]
fn hash_symbol() {
    assert_eq!(lex_one("#").kind, TokenKind::Hash);
}

#[test]
fn single_quote_is_error() {
    let tok = lex_one("'");
    assert_eq!(tok.kind, TokenKind::Error);
}

#[test]
fn unicode_line_comment_is_scanned_without_panicking() {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new("// It’s a UUID\n42", &mut interner);
    let token = lexer.next_token();

    assert_eq!(token.kind, TokenKind::IntLiteral);
    assert_eq!(token.span, Span::new(17, 19));
}

#[test]
fn unicode_block_comment_is_scanned_without_panicking() {
    let mut interner = Interner::new();
    let mut lexer = Lexer::new("/* keep — this */42", &mut interner);
    let token = lexer.next_token();

    assert_eq!(token.kind, TokenKind::IntLiteral);
    assert_eq!(token.span, Span::new(19, 21));
}
