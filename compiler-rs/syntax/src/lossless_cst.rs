// syntax/src/lossless_cst.rs

use infra::{Diagnostic, DiagnosticBag, Interner, Span};
use vfs::FileId;

use crate::cst::{CstNode, NodeId};
use crate::lex::Lexer;
use crate::lossless::{LosslessLexError, LosslessToken, LosslessTokenStream};
use crate::parse::{ParseError, Parser};

/// A half-open range into a lossless token stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRange {
    pub start: usize,
    pub end: usize,
}

impl TokenRange {
    /// Return whether this range contains no semantic tokens.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Errors raised while constructing a lossless CST snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LosslessCstError {
    Lex(LosslessLexError),
    InvalidToken {
        span: Span,
    },
    Parse {
        message: String,
        span: Span,
    },
    Diagnostics(Vec<Diagnostic>),
    InvalidNodeSpan {
        node: NodeId,
        span: Span,
        source_len: usize,
    },
    InvalidNodeId {
        index: usize,
        node: NodeId,
    },
}

/// An owned CST snapshot with source-preserving tokens associated to every node.
#[derive(Debug, Clone)]
pub struct LosslessCst {
    root: NodeId,
    nodes: Vec<CstNode>,
    tokens: LosslessTokenStream,
    token_ranges: Vec<TokenRange>,
}

impl LosslessCst {
    /// Return the root module node ID.
    pub const fn root(&self) -> NodeId {
        self.root
    }

    /// Return all parsed CST nodes in arena order.
    pub fn nodes(&self) -> &[CstNode] {
        &self.nodes
    }

    /// Return one CST node when its ID is within the arena.
    pub fn node(&self, id: NodeId) -> Option<&CstNode> {
        self.nodes.get(id.0 as usize)
    }

    /// Return the source-owned lossless token stream.
    pub const fn tokens(&self) -> &LosslessTokenStream {
        &self.tokens
    }

    /// Return the complete original source text.
    pub fn source(&self) -> &str {
        self.tokens.source()
    }

    /// Return the associated semantic-token range for one CST node.
    pub fn token_range(&self, id: NodeId) -> Option<TokenRange> {
        self.token_ranges.get(id.0 as usize).copied()
    }

    /// Return all semantic tokens covered by one CST node in source order.
    pub fn tokens_for(&self, id: NodeId) -> Option<&[LosslessToken]> {
        let range = self.token_range(id)?;
        self.tokens.tokens().get(range.start..range.end)
    }
}

/// Parse source and associate every CST node with its contiguous token range.
pub fn parse_lossless_cst(source: &str, file_id: FileId) -> Result<LosslessCst, LosslessCstError> {
    let token_stream = LosslessTokenStream::from_source(source).map_err(LosslessCstError::Lex)?;
    if let Some(token) = token_stream
        .tokens()
        .iter()
        .find(|token| token.kind == crate::grammar::TokenKind::Error)
    {
        return Err(LosslessCstError::InvalidToken { span: token.span });
    }
    let mut interner = Interner::new();
    let lexer = Lexer::new(source, &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, file_id);
    let root = parser.parse_file().map_err(parse_error_to_lossless_error)?;
    let parser_diagnostics = diagnostics.into_diagnostics();
    if !parser_diagnostics.is_empty() {
        return Err(LosslessCstError::Diagnostics(parser_diagnostics));
    }
    let token_ranges = associate_lossless_tokens(&nodes, &token_stream)?;
    Ok(LosslessCst {
        root,
        nodes,
        tokens: token_stream,
        token_ranges,
    })
}

/// Associate source-overlapping token ranges without mutating CST nodes.
pub fn associate_lossless_tokens(
    nodes: &[CstNode],
    tokens: &LosslessTokenStream,
) -> Result<Vec<TokenRange>, LosslessCstError> {
    let source = tokens.source();
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let expected = NodeId(index as u32);
            if node.id != expected {
                return Err(LosslessCstError::InvalidNodeId {
                    index,
                    node: node.id,
                });
            }
            validate_node_span(node, source)?;
            Ok(token_range_for_span(node.span, tokens.tokens()))
        })
        .collect()
}

/// Validate a node span before using it as a source slice or token boundary.
fn validate_node_span(node: &CstNode, source: &str) -> Result<(), LosslessCstError> {
    if node.span.start > node.span.end
        || node.span.end > source.len()
        || source.get(node.span.start..node.span.end).is_none()
    {
        return Err(LosslessCstError::InvalidNodeSpan {
            node: node.id,
            span: node.span,
            source_len: source.len(),
        });
    }
    Ok(())
}

/// Find the semantic token interval whose spans overlap a CST span.
fn token_range_for_span(span: Span, tokens: &[LosslessToken]) -> TokenRange {
    let start = tokens
        .iter()
        .position(|token| token.span.end > span.start)
        .unwrap_or(tokens.len());
    let end = tokens[start..]
        .iter()
        .position(|token| token.span.start >= span.end)
        .map_or(tokens.len(), |offset| start + offset);
    TokenRange { start, end }
}

/// Preserve parser error details at the lossless CST boundary.
fn parse_error_to_lossless_error(error: ParseError) -> LosslessCstError {
    LosslessCstError::Parse {
        message: error.message,
        span: error.span,
    }
}
