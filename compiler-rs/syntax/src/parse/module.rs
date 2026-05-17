// syntax/src/parse/module.rs

use super::{ParseError, Parser};
use crate::cst::{NodeId, NodeKind};
use crate::grammar::TokenKind;
use infra::Span;

impl<'a> Parser<'a> {
    pub fn parse_import(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        self.expect(TokenKind::LBrace)?;
        let mut items = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let name_span = self.current_span();
            let name = self.parse_identifier()?;
            let alias = if self.current_kind() == TokenKind::Eq {
                self.advance();
                Some(self.parse_identifier()?)
            } else {
                None
            };
            let item_span = if let Some(alias_id) = alias {
                Span::new(name_span.start, self.node_span(alias_id).end)
            } else {
                name_span
            };
            let mut item_children = vec![name];
            if let Some(alias_id) = alias {
                item_children.push(alias_id);
            }
            let item_node = self.push_node(
                NodeKind::ImportItem {
                    name: name_span,
                    alias,
                },
                item_span,
                item_children,
            );
            items.push(item_node);
            // No commas, just whitespace separation
        }
        self.expect(TokenKind::RBrace)?;
        self.expect(TokenKind::Eq)?;
        let source = self.parse_path()?;
        let span = Span::new(start, self.node_span(source).end);
        let mut children = vec![source];
        children.extend(items.clone());
        Ok(self.push_node(NodeKind::ImportDecl, span, children))
    }

    pub fn parse_export(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        self.expect(TokenKind::Exports)?;
        self.expect(TokenKind::LBrace)?;
        let mut items = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            items.push(self.parse_identifier()?);
        }
        let close = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(start, close.end);
        Ok(self.push_node(NodeKind::ExportDecl, span, items))
    }

    fn parse_path(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        match self.current_kind() {
            TokenKind::At => {
                self.advance(); // '@'
                if self.current_kind() == TokenKind::Slash {
                    self.advance(); // '/'
                }
                // scope
                if self.current_kind() == TokenKind::Identifier {
                    self.advance();
                } else {
                    return Err(self.error("expected scope name"));
                }
                // optional /pkg
                if self.current_kind() == TokenKind::Slash {
                    self.advance();
                    if self.current_kind() == TokenKind::Identifier {
                        self.advance();
                    } else {
                        return Err(self.error("expected package name"));
                    }
                }
                let end = self.current_span().start; // current token's start is the end of previous token
                // Actually we want the end of the last consumed token. Use current_span().end?
                // We'll use self.pos? Not available. We'll use self.current_span().end for now, but that gives the start of next token, which is fine.
                let span = Span::new(start, self.current_span().start);
                Ok(self.push_node(NodeKind::Ident(span), span, vec![]))
            }
            TokenKind::Dot => {
                self.advance();
                self.expect(TokenKind::Slash)?;
                let name = self.parse_identifier()?;
                let span = Span::new(start, self.node_span(name).end);
                Ok(self.push_node(NodeKind::Ident(span), span, vec![name]))
            }
            _ => Err(self.error("expected import source path")),
        }
    }
}
