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
                TokenKind::Eq => {
                    self.advance(); // consume '='
                    match self.current_kind() {
                        TokenKind::LParen if self.starts_function_definition() => {
                            return self.parse_function_def_after_name(
                                name,
                                name_span,
                                annotations,
                            );
                        }
                        TokenKind::Struct => {
                            self.advance();
                            return self.parse_struct_def_after_name(name, name_span, annotations);
                        }
                        TokenKind::Enum => {
                            self.advance();
                            return self.parse_enum_def_after_name(name, name_span, annotations);
                        }
                        _ => {
                            // Constant assignment: name = expr.
                            let value = self.parse_expr(0)?;
                            return Ok(self.make_assignment(annotations, name, None, value));
                        }
                    }
                }
                TokenKind::Colon => {
                    self.advance();
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
                            let type_annotation = self.parse_type()?;
                            self.expect(TokenKind::Eq)?;
                            let value = self.parse_expr(0)?;
                            return Ok(self.make_assignment(
                                annotations,
                                name,
                                Some(type_annotation),
                                value,
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

    /// Build a constant assignment while retaining an optional type annotation.
    fn make_assignment(
        &mut self,
        annotations: Vec<NodeId>,
        target: NodeId,
        type_annotation: Option<NodeId>,
        value: NodeId,
    ) -> NodeId {
        let span = Span::new(
            self.declaration_start(&annotations, self.node_span(target).start),
            self.node_span(value).end,
        );
        let mut children = annotations.clone();
        children.push(target);
        if let Some(type_id) = type_annotation {
            children.push(type_id);
        }
        children.push(value);
        self.push_node(
            NodeKind::AssignStmt {
                annotations,
                target,
                type_annotation,
                value,
            },
            span,
            children,
        )
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
        let fields = self.parse_struct_fields()?;
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

    /// Parse struct fields using the current Fer comma/newline separator rules.
    fn parse_struct_fields(&mut self) -> Result<Vec<NodeId>, ParseError> {
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
            if !self.consume_sequence_separator(end)? {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_enum_def_after_name(
        &mut self,
        name: NodeId,
        name_span: Span,
        annotations: Vec<NodeId>,
    ) -> Result<NodeId, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let variants = self.parse_enum_variants()?;
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

    /// Parse enum variants using the current Fer comma/newline separator rules.
    fn parse_enum_variants(&mut self) -> Result<Vec<NodeId>, ParseError> {
        let mut variants = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let variant = self.parse_identifier()?;
            let end = self.node_span(variant).end;
            variants.push(variant);
            if !self.consume_sequence_separator(end)? {
                break;
            }
        }
        Ok(variants)
    }

    /// Parse a block: `{ stmt* }`
    pub fn parse_block(&mut self) -> Result<NodeId, ParseError> {
        let open_span = self.current_span();
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let parsed = if self.current_kind() == TokenKind::LBrace {
                let expr = self.parse_expr(0)?;
                let span = self.node_span(expr);
                Ok(self.push_node(NodeKind::ExprStmt { expr }, span, vec![expr]))
            } else {
                self.parse_declaration()
            };
            match parsed {
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
        // Parse the required parameter list after the function binding operator.
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while self.current_kind() != TokenKind::RParen && self.current_kind() != TokenKind::Eof {
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

        // Parse the optional return type for compatibility with inferred internal functions.
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

    fn starts_function_definition(&mut self) -> bool {
        let checkpoint = self.checkpoint();
        let starts = if self.current_kind() != TokenKind::LParen {
            false
        } else {
            self.advance();
            if self.current_kind() == TokenKind::RParen {
                self.advance();
                matches!(self.current_kind(), TokenKind::Arrow | TokenKind::LBrace)
            } else {
                self.current_kind() == TokenKind::Identifier
                    && self.peek_kind() == Some(TokenKind::Colon)
            }
        };
        self.restore(checkpoint);
        starts
    }

    fn declaration_start(&self, annotations: &[NodeId], fallback: usize) -> usize {
        annotations
            .first()
            .map_or(fallback, |annotation| self.node_span(*annotation).start)
    }

    fn parse_type(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        match self.current_kind() {
            TokenKind::Struct => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let fields = self.parse_struct_fields()?;
                let close = self.current_span();
                self.expect(TokenKind::RBrace)?;
                Ok(self.push_node(
                    NodeKind::AnonymousStructType {
                        fields: fields.clone(),
                    },
                    Span::new(start, close.end),
                    fields,
                ))
            }
            TokenKind::Enum => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let variants = self.parse_enum_variants()?;
                let close = self.current_span();
                self.expect(TokenKind::RBrace)?;
                Ok(self.push_node(
                    NodeKind::AnonymousEnumType {
                        variants: variants.clone(),
                    },
                    Span::new(start, close.end),
                    variants,
                ))
            }
            _ => self.parse_identifier(),
        }
    }
}
