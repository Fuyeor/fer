// syntax/src/parse/string.rs

use infra::Span;

use crate::cst::{InterpolatedPart, NodeId, NodeKind};

use super::{ParseError, Parser};

impl<'a> Parser<'a> {
    /// Parse a tokenized interpolated string into text and expression parts.
    pub(super) fn parse_interpolated_string(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        self.expect(crate::grammar::TokenKind::StringStart)?;
        let mut parts = Vec::new();
        let mut children = Vec::new();
        while !matches!(
            self.current_kind(),
            crate::grammar::TokenKind::StringEnd | crate::grammar::TokenKind::Eof
        ) {
            match self.current_kind() {
                crate::grammar::TokenKind::StringPart => {
                    if let Some(symbol) = self.current_symbol()
                        && let Some(text) = self.lexer.symbol_text(symbol)
                    {
                        parts.push(InterpolatedPart::Text(text.to_owned()));
                    }
                    self.advance();
                }
                crate::grammar::TokenKind::ExprStart => {
                    self.advance();
                    let expression = self.parse_expr(0)?;
                    children.push(expression);
                    parts.push(InterpolatedPart::Expr(expression));
                    self.expect(crate::grammar::TokenKind::ExprEnd)?;
                }
                kind => {
                    return Err(
                        self.error(format!("unexpected token {kind:?} in interpolated string"))
                    );
                }
            }
        }
        let end = self.current_span().end;
        self.expect(crate::grammar::TokenKind::StringEnd)?;
        Ok(self.push_node(
            NodeKind::InterpolatedString { parts },
            Span::new(start, end),
            children,
        ))
    }
}
