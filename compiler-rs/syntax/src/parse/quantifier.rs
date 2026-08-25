// syntax/src/parse/quantifier.rs

use super::{ParseError, Parser};
use crate::cst::{NodeId, NodeKind, QuantifierKind};
use crate::grammar::TokenKind;
use infra::Span;

impl<'a> Parser<'a> {
    /// Identify quantifier names only in the `name (` expression context.
    pub(super) fn current_quantifier_kind(&self) -> Option<QuantifierKind> {
        if self.current_kind() != TokenKind::Identifier
            || self.peek_kind() != Some(TokenKind::LParen)
        {
            return None;
        }
        match self.lexer.source_text(self.current_span())? {
            "all" => Some(QuantifierKind::All),
            "any" => Some(QuantifierKind::Any),
            "one" => Some(QuantifierKind::One),
            "none" => Some(QuantifierKind::None),
            _ => None,
        }
    }

    /// Parse a quantifier condition list with comma or newline separators.
    pub(super) fn parse_quantifier(&mut self, kind: QuantifierKind) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        self.advance();
        self.expect(TokenKind::LParen)?;
        let mut conditions = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RParen | TokenKind::Eof) {
            let condition = self.parse_expr(0)?;
            let condition_end = self.node_span(condition).end;
            conditions.push(condition);
            if self.current_kind() == TokenKind::Comma {
                self.advance();
                continue;
            }
            if matches!(self.current_kind(), TokenKind::RParen | TokenKind::Eof) {
                break;
            }
            let has_newline = self
                .lexer
                .source_text(Span::new(condition_end, self.current_span().start))
                .is_some_and(|gap| gap.contains('\n'));
            if !has_newline {
                return Err(self.error("expected comma or newline between quantifier conditions"));
            }
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RParen)?;
        let span = Span::new(start, close_span.end);
        Ok(self.push_node(
            NodeKind::Quantifier {
                kind,
                conditions: conditions.clone(),
            },
            span,
            conditions,
        ))
    }
}
