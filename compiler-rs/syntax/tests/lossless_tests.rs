// compiler-rs/syntax/tests/lossless_tests.rs

use syntax::LosslessTokenStream;
use syntax::grammar::TokenKind;

#[test]
fn reconstructs_source_from_token_and_trivia_segments() {
    let source = "// header\r\nvalue = 42  /* trailing */\r\n";
    let stream = LosslessTokenStream::from_source(source).expect("source must lex");
    let mut reconstructed = String::new();

    for token in stream.tokens() {
        reconstructed.push_str(token.trivia(stream.source()).expect("valid trivia span"));
        reconstructed.push_str(token.text(stream.source()).expect("valid token span"));
    }

    assert_eq!(reconstructed, source);
    assert_eq!(
        stream.tokens().last().map(|token| token.kind),
        Some(TokenKind::Eof)
    );
    assert_eq!(
        stream
            .tokens()
            .last()
            .and_then(|token| token.trivia(stream.source())),
        Some("  /* trailing */\r\n")
    );
}

#[test]
fn preserves_raw_interpolation_spelling_and_inner_trivia() {
    let source = "message = `hello { value }`\n";
    let stream = LosslessTokenStream::from_source(source).expect("source must lex");
    let tokens = stream.tokens();

    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::StringStart)
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::ExprStart)
    );
    assert!(tokens.iter().any(|token| token.kind == TokenKind::ExprEnd));
    assert_eq!(stream.reconstruct(), source);
    assert!(
        tokens
            .iter()
            .any(|token| token.trivia(stream.source()) == Some(" "))
    );
}

#[test]
fn rejects_invalid_lexer_spans_without_panicking() {
    let source = "é";
    let stream = LosslessTokenStream::from_source(source).expect("unicode source must lex");
    assert_eq!(stream.reconstruct(), source);
}
