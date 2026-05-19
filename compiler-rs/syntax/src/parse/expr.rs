// syntax/src/parse/expr.rs

use super::{ParseError, Parser};
use crate::cst::{ChainStep, ChainStepKind, NodeId, NodeKind};
use crate::grammar::{self, Assoc, TokenKind};
use infra::Span;

impl<'a> Parser<'a> {
    /// Parse an expression with given precedence (Pratt parser).
    /// `min_prec` is the minimum precedence an operator must have to be
    /// parsed in this call (used for left-associativity).
    pub fn parse_expr(&mut self, min_prec: u8) -> Result<NodeId, ParseError> {
        // --- Prefix / Atom ---
        let mut lhs = self.parse_prefix()?;

        // --- Postfix / Infix loop ---
        loop {
            let kind = self.current_kind();
            // Postfix operations (call, field, index, match)
            match kind {
                TokenKind::LParen => {
                    // Function call: lhs(args)
                    lhs = self.parse_call(lhs)?;
                    continue;
                }
                TokenKind::Dot => {
                    // Start of a chain: lhs.field or lhs.method()
                    lhs = self.parse_chain(lhs)?;
                    continue;
                }
                TokenKind::LBracket => {
                    // Index expression: lhs[index]
                    lhs = self.parse_index(lhs)?;
                    continue;
                }
                TokenKind::LBrace if !self.suppress_match_postfix => {
                    lhs = self.parse_match_expr(lhs)?;
                    continue;
                }
                _ => {}
            }

            // Check for binary operators.
            if let Some((prec, assoc)) = grammar::prec_of(kind) {
                if prec < min_prec {
                    break; // lower precedence, stop here
                }
                let next_min = match assoc {
                    Assoc::Left => prec + 1,
                    Assoc::Right => prec,
                };
                let op_token = self.current;
                self.advance(); // consume operator
                let rhs = self.parse_expr(next_min)?;
                let span = Span::new(self.node_span(lhs).start, self.node_span(rhs).end);
                let bin = self.push_node(
                    NodeKind::BinaryOp {
                        op: op_token.span,
                        lhs,
                        rhs,
                    },
                    span,
                    vec![lhs, rhs],
                );
                lhs = bin;
                continue;
            }

            // Not a postfix or infix, exit loop.
            break;
        }

        Ok(lhs)
    }

    /// Parse a single match arm: `pattern { body }` or `{ body }` (default).
    fn parse_match_arm(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        let old_suppress = self.suppress_match_postfix;
        self.suppress_match_postfix = true;
        let pattern = if self.current_kind() == TokenKind::LBrace {
            None
        } else {
            Some(self.parse_match_pattern()?)
        };
        self.suppress_match_postfix = old_suppress;
        let body = self.parse_block()?;
        let span = Span::new(start, self.node_span(body).end);
        let mut children = Vec::new();
        if let Some(pat) = pattern {
            children.push(pat);
        }
        children.push(body);
        Ok(self.push_node(NodeKind::MatchArm { pattern, body }, span, children))
    }

    /// Parse a match expression: `scrutinee { arm* }`.
    fn parse_match_expr(&mut self, scrutinee: NodeId) -> Result<NodeId, ParseError> {
        let start = self.node_span(scrutinee).start;
        // The caller has already verified that the current token is '{'.
        self.advance(); // consume '{'
        let mut arms = Vec::new();
        while self.current_kind() != TokenKind::RBrace && self.current_kind() != TokenKind::Eof {
            let arm_id = self.parse_match_arm()?;
            arms.push(arm_id);
        }
        let close_span = self.current_span();
        self.expect(TokenKind::RBrace)?;
        let span = Span::new(start, close_span.end);
        let mut children = vec![scrutinee];
        for &arm_id in &arms {
            let arm_node = &self.nodes[arm_id.0 as usize];
            if let NodeKind::MatchArm { pattern, body } = &arm_node.kind {
                if let Some(pat) = pattern {
                    children.push(*pat);
                }
                children.push(*body);
            }
        }
        Ok(self.push_node(NodeKind::MatchExpr { scrutinee, arms }, span, children))
    }

    /// Parse the pattern part of a match arm.
    /// This can be:
    /// - A literal value (string, int, float, bool) – implicitly equals
    /// - A keyword condition (contains, matches, >, <, etc.) with operand
    /// - A full condition expression in parentheses
    fn parse_match_pattern(&mut self) -> Result<NodeId, ParseError> {
        let kind = self.current_kind();
        match kind {
            TokenKind::Contains
            | TokenKind::Less
            | TokenKind::More
            | TokenKind::Least
            | TokenKind::Most
            | TokenKind::Equals
            | TokenKind::Matches
            | TokenKind::Starts
            | TokenKind::Ends
            | TokenKind::In
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq => {
                let op_token = self.current;
                self.advance(); // consume operator
                let rhs = if op_token.kind == TokenKind::Matches {
                    if self.current_kind() != TokenKind::RegexLiteral {
                        self.lexer.set_regex_mode(false);
                        return Err(self.error("expected regex literal after matches"));
                    }
                    let regex_span = self.current_span();
                    let node = self.push_node(NodeKind::LitRegex, regex_span, vec![]);
                    self.advance(); // consume regex token
                    node
                } else {
                    self.parse_expr(0)?
                };
                let span = Span::new(op_token.span.start, self.node_span(rhs).end);
                Ok(self.push_node(
                    NodeKind::PatternCondition {
                        op: op_token.span,
                        rhs,
                    },
                    span,
                    vec![rhs],
                ))
            }
            TokenKind::LParen => self.parse_expr(0),
            _ => {
                let value = self.parse_expr(0)?;
                Ok(value)
            }
        }
    }

    /// Parse a prefix expression (atom or unary prefix).
    fn parse_prefix(&mut self) -> Result<NodeId, ParseError> {
        let token = self.current;
        match token.kind {
            TokenKind::Minus | TokenKind::Not => {
                self.advance(); // consume operator
                let operand = self.parse_prefix()?; // right-recursive for unary
                let span = Span::new(token.span.start, self.node_span(operand).end);
                Ok(self.push_node(
                    NodeKind::UnaryOp {
                        op: token.span,
                        expr: operand,
                    },
                    span,
                    vec![operand],
                ))
            }
            _ => self.parse_atom(),
        }
    }

    /// Parse a primary expression (no prefix operators).
    fn parse_atom(&mut self) -> Result<NodeId, ParseError> {
        let token = self.current;
        match token.kind {
            TokenKind::IntLiteral => {
                self.advance();
                Ok(self.push_node(NodeKind::LitInteger, token.span, vec![]))
            }
            TokenKind::FloatLiteral => {
                self.advance();
                Ok(self.push_node(NodeKind::LitFloat, token.span, vec![]))
            }
            TokenKind::StringLiteral => {
                self.advance();
                Ok(self.push_node(NodeKind::LitString, token.span, vec![]))
            }
            TokenKind::TrueKw | TokenKind::FalseKw => {
                let value = token.kind == TokenKind::TrueKw;
                self.advance();
                Ok(self.push_node(NodeKind::LitBool(value), token.span, vec![]))
            }
            TokenKind::Identifier => {
                self.advance();
                Ok(self.push_node(NodeKind::Ident(token.span), token.span, vec![]))
            }
            TokenKind::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_expr(0)?;
                self.expect(TokenKind::RParen)?;
                // We don't create a grouping node; just return inner expr.
                // Its span covers the parens? The inner expr's span does not include parens,
                // but the parentheses are just for grouping and need no CST node.
                Ok(expr)
            }
            _ => Err(self.error(format!("unexpected token {:?} in expression", token.kind))),
        }
    }

    // --- Postfix: call, index, chain ---

    fn parse_call(&mut self, func: NodeId) -> Result<NodeId, ParseError> {
        let open_paren = self.current_span();
        self.advance(); // consume '('
        let mut args = Vec::new();
        if self.current_kind() != TokenKind::RParen {
            loop {
                // Parse either a named argument (name = expr) or a positional argument (expr)
                let expr = self.parse_expr(0)?;
                if self.current_kind() == TokenKind::Eq {
                    // Named argument: the expression must be an identifier
                    let name_span = self.node_span(expr);
                    // We assume expr is an Ident node, but we could verify later.
                    self.advance(); // consume '='
                    let value = self.parse_expr(0)?;
                    let span = Span::new(name_span.start, self.node_span(value).end);
                    let named_arg = self.push_node(
                        NodeKind::NamedArg {
                            name: name_span,
                            value,
                        },
                        span,
                        vec![expr, value],
                    );
                    args.push(named_arg);
                } else {
                    // Positional argument
                    args.push(expr);
                }
                if self.current_kind() == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        let close_paren = self.current_span();
        self.expect(TokenKind::RParen)?;
        let span = Span::new(self.node_span(func).start, close_paren.end);
        let mut children = vec![func];
        children.extend(args.clone());
        Ok(self.push_node(NodeKind::Call { func, args }, span, children))
    }

    fn parse_index(&mut self, base: NodeId) -> Result<NodeId, ParseError> {
        let open_bracket = self.current_span();
        self.advance(); // '['
        let index = self.parse_expr(0)?;
        let close_bracket = self.current_span();
        self.expect(TokenKind::RBracket)?;
        let span = Span::new(self.node_span(base).start, close_bracket.end);
        Ok(self.push_node(
            NodeKind::Index {
                base,
                open_bracket,
                index,
                close_bracket,
            },
            span,
            vec![base, index],
        ))
    }

    fn parse_chain(&mut self, base: NodeId) -> Result<NodeId, ParseError> {
        // Already consumed the first '.', now parse steps.
        let mut steps = Vec::new();
        let mut current_base = base;
        // The dot we just consumed is for the first step.
        loop {
            if self.current_kind() != TokenKind::Dot {
                break;
            }
            let dot_span = self.current_span();
            self.advance(); // consume '.'
            let step = if self.current_kind() == TokenKind::Identifier
                && self.peek_kind() == Some(TokenKind::LParen)
            {
                // method call: .name(args)
                let name_token = self.current;
                self.advance(); // consume identifier
                // parse call args
                let open_paren = self.current_span();
                self.advance(); // '('
                let mut args = Vec::new();
                if self.current_kind() != TokenKind::RParen {
                    loop {
                        args.push(self.parse_expr(0)?);
                        if self.current_kind() == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                let close_paren = self.current_span();
                self.expect(TokenKind::RParen)?;
                ChainStep {
                    dot_token: dot_span,
                    kind: ChainStepKind::Call {
                        open_paren,
                        args,
                        close_paren,
                    },
                }
            } else if self.current_kind() == TokenKind::Identifier {
                // field access: .name
                let name_span = self.current_span();
                self.advance();
                ChainStep {
                    dot_token: dot_span,
                    kind: ChainStepKind::FieldAccess(name_span),
                }
            } else {
                return Err(self.error("expected field name or method call after '.'"));
            };
            steps.push(step);
            // Continue looping if there is another dot.
        }
        // Build ChainExpr node
        let span = Span::new(
            self.node_span(base).start,
            steps
                .last()
                .map(|s| step_span(s))
                .unwrap_or(self.node_span(base))
                .end,
        );
        let mut children = vec![base];
        // We don't store steps as children; they are stored in the node kind.
        let node = self.push_node(NodeKind::ChainExpr { base, steps }, span, children);
        Ok(node)
    }
}

fn step_span(step: &ChainStep) -> Span {
    // A rough approximation: start of dot to end of step's last token.
    match &step.kind {
        ChainStepKind::FieldAccess(span) => Span::new(step.dot_token.start, span.end),
        ChainStepKind::Call { close_paren, .. } => Span::new(step.dot_token.start, close_paren.end),
        ChainStepKind::Index { close_bracket, .. } => {
            Span::new(step.dot_token.start, close_bracket.end)
        }
    }
}
