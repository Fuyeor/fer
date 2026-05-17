// syntax/src/parse/stmt.rs

use super::{ParseError, Parser};
use crate::cst::{NodeId, NodeKind};
use crate::grammar::TokenKind;
use infra::Span;

impl<'a> Parser<'a> {
    pub fn parse_declaration(&mut self) -> Result<NodeId, ParseError> {
        // Check for import or export
        if self.current_kind() == TokenKind::LBrace {
            return self.parse_import();
        }
        if self.current_kind() == TokenKind::Exports {
            return self.parse_export();
        }

        if self.current_kind() == TokenKind::Identifier {
            let ck = self.checkpoint(); // save state before consuming identifier
            let name_span = self.current_span();
            let name = self.parse_identifier()?;

            match self.current_kind() {
                TokenKind::LParen | TokenKind::Colon => {
                    // Try function definition; if fails, restore and parse as expression
                    match self.parse_function_def_after_name(name, name_span) {
                        Ok(node) => return Ok(node),
                        Err(_) => {
                            self.restore(ck);
                            // fall through to expression parsing below
                        }
                    }
                }
                TokenKind::Eq => {
                    self.advance(); // consume '='
                    match self.current_kind() {
                        TokenKind::Struct => {
                            self.advance();
                            return self.parse_struct_def_after_name(name, name_span);
                        }
                        TokenKind::Enum => {
                            self.advance();
                            return self.parse_enum_def_after_name(name, name_span);
                        }
                        _ => {
                            // constant assignment: name = expr
                            let value = self.parse_expr(0)?;
                            let span = Span::new(name_span.start, self.node_span(value).end);
                            return Ok(self.push_node(
                                NodeKind::AssignStmt {
                                    target: name,
                                    value,
                                },
                                span,
                                vec![name, value],
                            ));
                        }
                    }
                }
                _ => {
                    // Not a declaration start, restore and parse as expression
                    self.restore(ck);
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
    ) -> Result<NodeId, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let field_name_span = self.current_span();
            let field_name = self.parse_identifier()?;
            self.expect(TokenKind::Eq)?; // field = type
            let field_type = self.parse_type()?;
            let field_span = Span::new(field_name_span.start, self.node_span(field_type).end);
            let field_node = self.push_node(
                NodeKind::FieldDef {
                    name: field_name_span,
                    type_annotation: field_type,
                },
                field_span,
                vec![field_name, field_type],
            );
            fields.push(field_node);
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(name_span.start, close_span.end);
        let mut children = vec![name];
        children.extend(fields.clone());
        Ok(self.push_node(
            NodeKind::StructDef {
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
    ) -> Result<NodeId, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let variant_name = self.parse_identifier()?;
            variants.push(variant_name);
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(name_span.start, close_span.end);
        let mut children = vec![name];
        children.extend(variants.clone());
        Ok(self.push_node(
            NodeKind::EnumDef {
                name: name_span,
                variants,
            },
            span,
            children,
        ))
    }

    fn parse_assign_stmt(&mut self) -> Result<NodeId, ParseError> {
        let target = self.parse_expr(0)?; // will parse the identifier
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr(0)?;
        let span = Span::new(self.node_span(target).start, self.node_span(value).end);
        Ok(self.push_node(
            NodeKind::AssignStmt { target, value },
            span,
            vec![target, value],
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
                Err(e) => {
                    // Error recovery: skip to next synchronization point.
                    self.skip_until(&[TokenKind::RBrace, TokenKind::Struct, TokenKind::Enum]);
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

        let span = Span::new(name_span.start, self.node_span(body).end);

        let mut children = vec![name];
        children.extend(params.clone());
        if let Some(rt) = return_type {
            children.push(rt);
        }
        children.push(body);

        Ok(self.push_node(
            NodeKind::FunctionDef {
                name: name_span,
                params,
                return_type,
                body,
            },
            span,
            children,
        ))
    }

    fn parse_struct_def(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        self.advance(); // 'struct'
        let name_span = self.current_span();
        let name = self.parse_identifier()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let field_name = self.parse_identifier()?;
            self.expect(TokenKind::Eq)?;
            let field_type = self.parse_type()?;
            // We'll create a temporary node for field (or skip for now)
            fields.push(field_name); // placeholder
            if self.current_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(start, close_span.end);
        // children: name, field nodes...
        let mut children = vec![name];
        children.extend(fields.clone());
        Ok(self.push_node(
            NodeKind::StructDef {
                name: name_span,
                fields,
            },
            span,
            children,
        ))
    }

    fn parse_enum_def(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        self.advance(); // 'enum'
        let name_span = self.current_span();
        let name = self.parse_identifier()?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let variant_name = self.parse_identifier()?;
            variants.push(variant_name);
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(start, close_span.end);
        let children = vec![name]; // variants are identifiers, not nodes? We'll push them as Ident nodes.
        Ok(self.push_node(
            NodeKind::EnumDef {
                name: name_span,
                variants,
            },
            span,
            children,
        ))
    }

    fn parse_type(&mut self) -> Result<NodeId, ParseError> {
        // For now, type is just an identifier.
        self.parse_identifier()
    }
}
