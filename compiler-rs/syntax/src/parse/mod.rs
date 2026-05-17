// syntax/src/parse/mod.rs

pub mod error;
pub mod expr;
pub mod module;
pub mod pattern;
pub mod stmt;

use infra::{DiagnosticBag, Span};
use vfs::FileId;

use crate::cst::{CstNode, NodeId, NodeKind};
use crate::grammar::TokenKind;
use crate::lex::{Lexer, LexerCheckpoint, Token};

/// A recoverable parse error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// The recursive descent parser.
pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    pub nodes: &'a mut Vec<CstNode>,
    pub diagnostics: &'a mut DiagnosticBag,
    pub file_id: FileId,
    /// Current token (the one that will be consumed by `advance`).
    current: Token,
    peek: Option<Token>, // one-token lookahead
}

struct ParserCheckpoint {
    current: Token,
    peek: Option<Token>,
    lexer_ck: LexerCheckpoint,
}

impl<'a> Parser<'a> {
    pub fn new(
        mut lexer: Lexer<'a>,
        nodes: &'a mut Vec<CstNode>,
        diagnostics: &'a mut DiagnosticBag,
        file_id: FileId,
    ) -> Self {
        let current = lexer.next_token(); // prime the first token
        let peek = Some(lexer.next_token());
        Self {
            lexer,
            nodes,
            diagnostics,
            file_id,
            current,
            peek,
        }
    }

    /// Consume the current token and advance to the next one.
    pub fn advance(&mut self) {
        self.current = self.peek.take().unwrap_or_else(|| {
            // If peek was None, generate Eof.
            Token {
                kind: TokenKind::Eof,
                span: Span::dummy(),
                symbol: None,
            }
        });
        self.peek = Some(self.lexer.next_token());
    }

    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek.map(|t| t.kind)
    }

    /// Return the kind of the current token.
    pub fn current_kind(&self) -> TokenKind {
        self.current.kind
    }

    /// Return the span of the current token.
    pub fn current_span(&self) -> Span {
        self.current.span
    }

    /// Return the symbol of the current token (if it has one).
    pub fn current_symbol(&self) -> Option<infra::Symbol> {
        self.current.symbol
    }

    /// Allocate a new CST node and return its ID.
    pub fn push_node(&mut self, kind: NodeKind, span: Span, children: Vec<NodeId>) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(CstNode {
            id,
            kind,
            span,
            children,
        });
        id
    }

    /// Parse the whole file, returning the root module node.
    pub fn parse_file(&mut self) -> Result<NodeId, ParseError> {
        let start = self.current_span().start;
        let mut stmts = Vec::new();
        // Check for header comment (/// @/...)
        // We'll skip it for now; the comment is not yet a token in our lexer.
        while self.current_kind() != TokenKind::Eof {
            match self.parse_declaration() {
                Ok(stmt) => stmts.push(stmt),
                Err(_) => {
                    // Attempt recovery by skipping to next possible declaration start.
                    self.skip_until(&[TokenKind::Struct, TokenKind::Enum, TokenKind::Exports]);
                    if self.current_kind() == TokenKind::Eof {
                        break;
                    }
                }
            }
        }
        let span = Span::new(start, self.current_span().end);
        Ok(self.push_node(NodeKind::Module, span, stmts))
    }

    fn checkpoint(&self) -> ParserCheckpoint {
        ParserCheckpoint {
            current: self.current,
            peek: self.peek,
            lexer_ck: self.lexer.checkpoint(),
        }
    }

    fn restore(&mut self, ck: ParserCheckpoint) {
        self.current = ck.current;
        self.peek = ck.peek;
        self.lexer.restore(ck.lexer_ck);
    }

    /// Return the span of a previously pushed node.
    pub(crate) fn node_span(&self, id: NodeId) -> Span {
        self.nodes[id.0 as usize].span
    }

    /// Parse an identifier and push it as a CST node.
    pub(crate) fn parse_identifier(&mut self) -> Result<NodeId, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Identifier)?;
        Ok(self.push_node(NodeKind::Ident(span), span, vec![]))
    }
}
