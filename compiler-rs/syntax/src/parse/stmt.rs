// syntax/src/parse/stmt.rs

use super::{ParseError, Parser};
use crate::cst::{NodeId, NodeKind};
use crate::grammar::TokenKind;
use infra::Span;

impl<'a> Parser<'a> {
    pub fn parse_declaration(&mut self) -> Result<NodeId, ParseError> {
        let annotations = self.parse_annotations()?;
        // Check for import or export
        if self.current_kind() == TokenKind::LBrace {
            if !annotations.is_empty() {
                return Err(self.error("annotations are not supported on imports"));
            }
            return self.parse_import();
        }
        if self.current_kind() == TokenKind::Exports {
            if !annotations.is_empty() {
                return Err(self.error("annotations are not supported on exports"));
            }
            return self.parse_export();
        }
        if !annotations.is_empty() && self.current_kind() != TokenKind::Identifier {
            return Err(self.error("expected a declaration after annotation"));
        }

        if self.current_kind() == TokenKind::Identifier {
            let ck = self.checkpoint(); // save state before consuming identifier
            let name_span = self.current_span();
            let name = self.parse_identifier()?;

            match self.current_kind() {
                TokenKind::LParen | TokenKind::Colon => {
                    // Try a function definition; unannotated forms may fall back to expressions
                    match self.parse_function_def_after_name(name, name_span, annotations.clone()) {
                        Ok(node) => return Ok(node),
                        Err(_) => {
                            self.restore(ck);
                            if !annotations.is_empty() {
                                return Err(
                                    self.error("expected a function declaration after annotation")
                                );
                            }
                            // fall through to expression parsing below
                        }
                    }
                }
                TokenKind::Eq => {
                    self.advance(); // consume '='
                    match self.current_kind() {
                        TokenKind::Struct => {
                            self.advance();
                            return self.parse_struct_def_after_name(name, name_span, annotations);
                        }
                        TokenKind::Enum => {
                            self.advance();
                            return self.parse_enum_def_after_name(name, name_span, annotations);
                        }
                        _ => {
                            // constant assignment: name = expr
                            let value = self.parse_expr(0)?;
                            let span = Span::new(
                                self.declaration_start(&annotations, name_span.start),
                                self.node_span(value).end,
                            );
                            let mut children = annotations.clone();
                            children.extend([name, value]);
                            return Ok(self.push_node(
                                NodeKind::AssignStmt {
                                    annotations,
                                    target: name,
                                    value,
                                },
                                span,
                                children,
                            ));
                        }
                    }
                }
                _ => {
                    // Not a declaration start, restore and parse as expression
                    self.restore(ck);
                    if !annotations.is_empty() {
                        return Err(self.error("expected a declaration after annotation"));
                    }
                }
            }
        }

        // Fallback: expression statement
        let expr = self.parse_expr(0)?;
        let span = self.node_span(expr);
        Ok(self.push_node(NodeKind::ExprStmt { expr }, span, vec![expr]))
    }

    /// Parse a statement inside a block (expression or assignment).
    pub fn parse_stmt(&mut self) -> Result<NodeId, ParseError> {
        self.parse_declaration()
    }

    fn parse_struct_def_after_name(
        &mut self,
        name: NodeId,
        name_span: Span,
        annotations: Vec<NodeId>,
    ) -> Result<NodeId, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let field_annotations = self.parse_annotations()?;
            let field_name_span = self.current_span();
            let field_name = self.parse_identifier()?;
            let type_annotation = if self.current_kind() == TokenKind::Colon {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            let default_value = if self.current_kind() == TokenKind::Eq {
                self.advance();
                Some(self.parse_expr(0)?)
            } else {
                None
            };
            if type_annotation.is_none() && default_value.is_none() {
                return Err(self.error("expected a field type or default value"));
            }
            let end = default_value
                .map(|value| self.node_span(value).end)
                .or_else(|| type_annotation.map(|type_id| self.node_span(type_id).end))
                .unwrap_or(field_name_span.end);
            let field_span = Span::new(
                self.declaration_start(&field_annotations, field_name_span.start),
                end,
            );
            let mut children = field_annotations.clone();
            children.push(field_name);
            if let Some(type_id) = type_annotation {
                children.push(type_id);
            }
            if let Some(value_id) = default_value {
                children.push(value_id);
            }
            let field_node = self.push_node(
                NodeKind::FieldDef {
                    annotations: field_annotations,
                    name: field_name_span,
                    type_annotation,
                    default_value,
                },
                field_span,
                children,
            );
            fields.push(field_node);
            if self.current_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(
            self.declaration_start(&annotations, name_span.start),
            close_span.end,
        );
        let mut children = annotations.clone();
        children.push(name);
        children.extend(fields.iter().copied());
        Ok(self.push_node(
            NodeKind::StructDef {
                annotations,
                name: name_span,
                fields,
            },
            span,
            children,
        ))
    }

    fn parse_enum_def_after_name(
        &mut self,
        name: NodeId,
        name_span: Span,
        annotations: Vec<NodeId>,
    ) -> Result<NodeId, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let variant_name = self.parse_identifier()?;
            variants.push(variant_name);
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(
            self.declaration_start(&annotations, name_span.start),
            close_span.end,
        );
        let mut children = annotations.clone();
        children.push(name);
        children.extend(variants.iter().copied());
        Ok(self.push_node(
            NodeKind::EnumDef {
                annotations,
                name: name_span,
                variants,
            },
            span,
            children,
        ))
    }

    /// Parse a block: `{ stmt* }`
    pub fn parse_block(&mut self) -> Result<NodeId, ParseError> {
        let open_span = self.current_span();
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            match self.parse_declaration() {
                Ok(stmt) => stmts.push(stmt),
                Err(_) => {
                    // Error recovery: skip to next synchronization point.
                    self.skip_until(&[
                        TokenKind::Hash,
                        TokenKind::Identifier,
                        TokenKind::RBrace,
                        TokenKind::Struct,
                        TokenKind::Enum,
                    ]);
                    if self.current_kind() == TokenKind::RBrace {
                        break;
                    }
                }
            }
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(open_span.start, close_span.end);
        Ok(self.push_node(
            NodeKind::Block {
                statements: stmts.clone(),
            },
            span,
            stmts,
        ))
    }

    fn parse_function_def_after_name(
        &mut self,
        name: NodeId,
        name_span: Span,
        annotations: Vec<NodeId>,
    ) -> Result<NodeId, ParseError> {
        // Parse parameter list
        let mut params = Vec::new();
        if self.current_kind() == TokenKind::LParen {
            self.advance();
            while self.current_kind() != TokenKind::RParen && self.current_kind() != TokenKind::Eof
            {
                let param_name_span = self.current_span();
                let param_name = self.parse_identifier()?;
                self.expect(TokenKind::Colon)?;
                let param_type = self.parse_type()?;
                let param_span = Span::new(param_name_span.start, self.node_span(param_type).end);
                let param_node = self.push_node(
                    NodeKind::Param {
                        name: param_name_span,
                        type_annotation: param_type,
                    },
                    param_span,
                    vec![param_name, param_type],
                );
                params.push(param_node);
                if self.current_kind() == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
        }

        // Return type (optional)
        let return_type = if self.current_kind() == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Body
        let body = self.parse_block()?;

        let span = Span::new(
            self.declaration_start(&annotations, name_span.start),
            self.node_span(body).end,
        );

        let mut children = annotations.clone();
        children.push(name);
        children.extend(params.clone());
        if let Some(rt) = return_type {
            children.push(rt);
        }
        children.push(body);

        Ok(self.push_node(
            NodeKind::FunctionDef {
                annotations,
                name: name_span,
                params,
                return_type,
                body,
            },
            span,
            children,
        ))
    }

    fn declaration_start(&self, annotations: &[NodeId], fallback: usize) -> usize {
        annotations
            .first()
            .map_or(fallback, |annotation| self.node_span(*annotation).start)
    }

    fn parse_type(&mut self) -> Result<NodeId, ParseError> {
        // For now, type is just an identifier.
        self.parse_identifier()
    }
}
