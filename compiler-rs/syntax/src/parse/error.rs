// syntax/src/parse/error.rs

use super::{ParseError, Parser};
use crate::grammar::TokenKind;
use infra::Diagnostic;

impl<'a> Parser<'a> {
    /// Emit an error diagnostic and return a ParseError.
    pub fn error(&mut self, message: impl Into<String>) -> ParseError {
        let msg = message.into();
        let span = self.current_span();
        self.diagnostics
            .add(Diagnostic::error("parse-error", msg.clone(), span));
        ParseError { message: msg, span }
    }

    /// If the current token is of the expected kind, consume it and return Ok.
    /// Otherwise, emit an error and return Err(ParseError).
    pub fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.current_kind() == kind {
            self.advance();
            Ok(())
        } else {
            let msg = format!("expected {:?}, found {:?}", kind, self.current_kind());
            Err(self.error(msg))
        }
    }

    /// Consume tokens until we see one of the synchronization tokens,
    /// then stop. This implements error recovery so parsing can continue.
    pub fn skip_until(&mut self, sync_tokens: &[TokenKind]) {
        while self.current_kind() != TokenKind::Eof {
            if sync_tokens.contains(&self.current_kind()) {
                break;
            }
            self.advance();
        }
    }
}
