// syntax/src/lossless.rs

use infra::{Interner, Span};

use crate::grammar::TokenKind;
use crate::lex::Lexer;

/// One semantic token together with the exact trivia preceding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LosslessToken {
    pub kind: TokenKind,
    pub span: Span,
    pub leading_trivia: Span,
}

impl LosslessToken {
    /// Return the token's original source spelling when the span is valid.
    pub fn text<'a>(&self, source: &'a str) -> Option<&'a str> {
        source.get(self.span.start..self.span.end)
    }

    /// Return the exact whitespace/comment text preceding this token.
    pub fn trivia<'a>(&self, source: &'a str) -> Option<&'a str> {
        source.get(self.leading_trivia.start..self.leading_trivia.end)
    }
}

/// Errors raised while converting the existing lexer stream into a lossless stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LosslessLexError {
    InvalidTokenSpan { span: Span, source_len: usize },
    InvalidTokenOrder { previous: Span, current: Span },
    InvalidUtf8Boundary { span: Span },
}

/// A source-owned token stream that can reproduce its input byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessTokenStream {
    source: String,
    tokens: Vec<LosslessToken>,
}

impl LosslessTokenStream {
    /// Lex source while retaining every gap between semantic tokens as trivia.
    pub fn from_source(source: &str) -> Result<Self, LosslessLexError> {
        let mut interner = Interner::new();
        let mut lexer = Lexer::new(source, &mut interner);
        let mut tokens = Vec::new();
        let mut cursor = 0;

        loop {
            let token = lexer.next_token();
            validate_span(source, token.span)?;
            if token.span.start < cursor {
                return Err(LosslessLexError::InvalidTokenOrder {
                    previous: Span::new(cursor, cursor),
                    current: token.span,
                });
            }
            let leading_trivia = Span::new(cursor, token.span.start);
            validate_span(source, leading_trivia)?;
            tokens.push(LosslessToken {
                kind: token.kind,
                span: token.span,
                leading_trivia,
            });
            cursor = token.span.end;
            if token.kind == TokenKind::Eof {
                break;
            }
        }

        Ok(Self {
            source: source.to_owned(),
            tokens,
        })
    }

    /// Return the complete original source owned by this stream.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return all tokens, including the trailing-trivia-bearing EOF token.
    pub fn tokens(&self) -> &[LosslessToken] {
        &self.tokens
    }

    /// Return an arbitrary valid source slice for formatter consumers.
    pub fn slice(&self, span: Span) -> Option<&str> {
        self.source.get(span.start..span.end)
    }

    /// Reconstruct the input without consulting token semantics.
    pub fn reconstruct(&self) -> String {
        self.source.clone()
    }
}

/// Validate both byte bounds and UTF-8 boundaries before exposing a span.
fn validate_span(source: &str, span: Span) -> Result<(), LosslessLexError> {
    if span.start > span.end || span.end > source.len() {
        return Err(LosslessLexError::InvalidTokenSpan {
            span,
            source_len: source.len(),
        });
    }
    if source.get(span.start..span.end).is_none() {
        return Err(LosslessLexError::InvalidUtf8Boundary { span });
    }
    Ok(())
}
