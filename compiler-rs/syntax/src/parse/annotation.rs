// syntax/src/parse/annotation.rs
use super::{ParseError, Parser};
use crate::cst::{NodeId, NodeKind};
use crate::grammar::TokenKind;
use infra::Span;

impl<'a> Parser<'a> {
    /// Parse zero or more `#[name]` or `#[name = value]` prefixes.
    pub(crate) fn parse_annotations(&mut self) -> Result<Vec<NodeId>, ParseError> {
        let mut annotations = Vec::new();
        while self.current_kind() == TokenKind::Hash {
            let start = self.current_span().start;
            self.advance();
            self.expect(TokenKind::LBracket)?;
            let name_span = self.current_span();
            let name = self.parse_identifier()?;
            let mut arguments = Vec::new();
            if self.current_kind() != TokenKind::RBracket {
                if self.current_kind() == TokenKind::Eq {
                    self.advance();
                }
                arguments.push(self.parse_annotation_argument(None)?);
                while self.current_kind() == TokenKind::Comma {
                    self.advance();
                    if self.current_kind() == TokenKind::RBracket {
                        break;
                    }
                    let key = if self.current_kind() == TokenKind::Identifier
                        && self.peek_kind() == Some(TokenKind::Eq)
                    {
                        let key_span = self.current_span();
                        self.advance();
                        self.advance();
                        Some(key_span)
                    } else {
                        None
                    };
                    arguments.push(self.parse_annotation_argument(key)?);
                }
            }
            let close_span = self.current_span();
            self.expect(TokenKind::RBracket)?;
            let span = Span::new(start, close_span.end);
            let mut children = vec![name];
            children.extend(arguments.iter().copied());
            annotations.push(self.push_node(
                NodeKind::Annotation {
                    name: name_span,
                    arguments,
                },
                span,
                children,
            ));
        }
        Ok(annotations)
    }

    /// Parse one annotation argument after its optional name and equals token.
    fn parse_annotation_argument(&mut self, name: Option<Span>) -> Result<NodeId, ParseError> {
        let value = self.parse_expr(0)?;
        let span = Span::new(
            name.map_or(self.node_span(value).start, |span| span.start),
            self.node_span(value).end,
        );
        Ok(self.push_node(NodeKind::AnnotationArg { name, value }, span, vec![value]))
    }
}
