// syntax/src/formatter/spacing.rs

use infra::Span;

use crate::grammar::TokenKind;
use crate::lossless::{LosslessToken, LosslessTokenStream};

/// Rewrite only horizontal trivia between adjacent semantic tokens.
pub(super) fn rewrite_token_spacing(
    source: &str,
    stream: &LosslessTokenStream,
    opaque_ranges: &[Span],
) -> String {
    let tokens = stream.tokens();
    let mut edits = Vec::new();
    for index in 1..tokens.len() {
        let previous = &tokens[index - 1];
        let current = &tokens[index];
        if current.kind == TokenKind::Eof {
            continue;
        }
        let gap = Span::new(previous.span.end, current.span.start);
        let Some(gap_text) = source.get(gap.start..gap.end) else {
            continue;
        };
        if !is_horizontal_gap(gap_text) || overlaps_opaque(gap, opaque_ranges) {
            continue;
        }
        let Some(desired) = spacing_between(tokens, index, source) else {
            continue;
        };
        if gap_text != desired {
            edits.push(SpacingEdit {
                span: gap,
                replacement: desired,
            });
        }
    }

    let mut output = source.to_owned();
    for edit in edits.into_iter().rev() {
        output.replace_range(edit.span.start..edit.span.end, edit.replacement);
    }
    output
}

#[derive(Debug, Clone, Copy)]
struct SpacingEdit {
    span: Span,
    replacement: &'static str,
}

/// Return whether a gap is safe to canonicalize without touching line layout or comments.
fn is_horizontal_gap(gap: &str) -> bool {
    gap.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

/// Avoid changing the interior of opaque strings and interpolation bodies.
fn overlaps_opaque(gap: Span, opaque_ranges: &[Span]) -> bool {
    opaque_ranges
        .iter()
        .any(|range| range.start < gap.end && gap.start < range.end)
}

/// Choose canonical spacing for one token boundary; `None` preserves the gap exactly.
fn spacing_between(tokens: &[LosslessToken], index: usize, source: &str) -> Option<&'static str> {
    let previous = &tokens[index - 1];
    let current = &tokens[index];
    if is_path_separator(tokens, index - 1)
        || is_path_separator(tokens, index)
        || previous.kind == TokenKind::At
    {
        return Some("");
    }
    if previous.kind == TokenKind::LBrace && current.kind == TokenKind::RBrace {
        return Some("");
    }
    if current.kind == TokenKind::RBrace {
        return Some(" ");
    }
    if current.kind == TokenKind::LBrace {
        return if matches!(previous.kind, TokenKind::LParen | TokenKind::LBracket) {
            Some("")
        } else {
            Some(" ")
        };
    }
    if current.kind == TokenKind::LParen {
        return if previous.kind == TokenKind::LBrace
            || previous.kind == TokenKind::Comma
            || previous.kind == TokenKind::Colon
            || is_binary_operator(previous.kind, tokens, index - 1)
            || needs_space_before_lparen(previous, source)
        {
            Some(" ")
        } else {
            Some("")
        };
    }
    if previous.kind == TokenKind::LBrace {
        return Some(" ");
    }
    if current.kind == TokenKind::Dot
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::Slash)
    {
        return if previous.kind == TokenKind::Eq {
            Some(" ")
        } else {
            Some("")
        };
    }
    if matches!(
        current.kind,
        TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::Dot
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::LBracket
    ) {
        return Some("");
    }
    if matches!(
        previous.kind,
        TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot
    ) {
        return Some("");
    }
    if previous.kind == TokenKind::Comma || previous.kind == TokenKind::Colon {
        return Some(" ");
    }
    if is_binary_operator(previous.kind, tokens, index - 1)
        || is_binary_operator(current.kind, tokens, index)
    {
        return Some(" ");
    }
    if previous.kind == TokenKind::Minus && is_unary_prefix(tokens, index - 1) {
        return Some("");
    }
    if is_word_like(previous.kind) && is_word_like(current.kind) {
        return Some(" ");
    }
    if previous.kind == TokenKind::RBracket && is_word_like(current.kind) {
        return Some(" ");
    }
    None
}

/// Distinguish ordinary calls from the formal spacing of quantifiers and `not (`.
fn needs_space_before_lparen(previous: &LosslessToken, source: &str) -> bool {
    if previous.kind == TokenKind::Not {
        return true;
    }
    previous.kind == TokenKind::Identifier
        && previous
            .text(source)
            .is_some_and(|text| matches!(text, "all" | "any" | "one" | "none"))
}

/// Recognize arithmetic/comparison operators while treating unary minus as a prefix operator.
fn is_binary_operator(kind: TokenKind, tokens: &[LosslessToken], index: usize) -> bool {
    if kind != TokenKind::Minus {
        return matches!(
            kind,
            TokenKind::Eq
                | TokenKind::Arrow
                | TokenKind::Plus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::LtEq
                | TokenKind::GtEq
                | TokenKind::Less
                | TokenKind::More
                | TokenKind::Least
                | TokenKind::Most
                | TokenKind::Equals
                | TokenKind::Contains
                | TokenKind::In
                | TokenKind::Matches
                | TokenKind::Starts
                | TokenKind::Ends
        );
    }
    !is_unary_prefix(tokens, index)
}

/// Determine whether a minus token begins an expression rather than joining two operands.
fn is_unary_prefix(tokens: &[LosslessToken], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    matches!(
        tokens[index - 1].kind,
        TokenKind::LBrace
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Eq
            | TokenKind::Arrow
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
            | TokenKind::Less
            | TokenKind::More
            | TokenKind::Least
            | TokenKind::Most
            | TokenKind::Equals
            | TokenKind::Contains
            | TokenKind::In
            | TokenKind::Matches
            | TokenKind::Starts
            | TokenKind::Ends
            | TokenKind::Not
    )
}

/// Treat literals, identifiers, keywords, and type-like names as word-shaped tokens.
fn is_word_like(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::IntLiteral
            | TokenKind::FloatLiteral
            | TokenKind::StringLiteral
            | TokenKind::TrueKw
            | TokenKind::FalseKw
            | TokenKind::RegexLiteral
            | TokenKind::Identifier
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Exports
            | TokenKind::Not
            | TokenKind::Contains
            | TokenKind::Less
            | TokenKind::More
            | TokenKind::Least
            | TokenKind::Most
            | TokenKind::Equals
            | TokenKind::In
            | TokenKind::Matches
            | TokenKind::Starts
            | TokenKind::Ends
    )
}

/// Identify slash tokens that belong to an import path rather than arithmetic.
fn is_path_separator(tokens: &[LosslessToken], index: usize) -> bool {
    if tokens
        .get(index)
        .is_none_or(|token| token.kind != TokenKind::Slash)
    {
        return false;
    }
    if tokens
        .get(index.wrapping_sub(1))
        .is_some_and(|token| matches!(token.kind, TokenKind::At | TokenKind::Dot))
    {
        return true;
    }
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        match tokens[cursor].kind {
            TokenKind::Identifier | TokenKind::Slash | TokenKind::Dot => {}
            TokenKind::At => return true,
            _ => return false,
        }
    }
    false
}
