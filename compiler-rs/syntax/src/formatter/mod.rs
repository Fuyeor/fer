// syntax/src/formatter/mod.rs

use infra::Span;
use vfs::FileId;

use crate::grammar::TokenKind;
use crate::lossless::{LosslessLexError, LosslessTokenStream};
use crate::lossless_cst::{LosslessCstError, parse_lossless_cst};

mod spacing;
use spacing::rewrite_token_spacing;

/// Formatting options for the conservative source formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent_width: usize,
    pub use_tabs: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_width: 2,
            use_tabs: false,
        }
    }
}

/// Errors that prevent the formatter from producing a safe whitespace edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    Lex(LosslessLexError),
    Parse { span: Span, message: String },
    InvalidIndentWidth { width: usize },
    InvalidToken { span: Span },
    UnbalancedBraces { span: Span },
}

/// Format a Fer source string using the default two-space indentation.
pub fn format_source(source: &str) -> Result<String, FormatError> {
    format_source_with_options(source, FormatOptions::default())
}

/// Format indentation and safe token-boundary spacing while preserving opaque source bytes.
pub fn format_source_with_options(
    source: &str,
    options: FormatOptions,
) -> Result<String, FormatError> {
    if options.indent_width == 0 {
        return Err(FormatError::InvalidIndentWidth {
            width: options.indent_width,
        });
    }
    let source_stream = LosslessTokenStream::from_source(source).map_err(FormatError::Lex)?;
    let (events, opaque_ranges) = collect_layout_metadata(&source_stream)?;
    let _cst = parse_lossless_cst(source, FileId(0)).map_err(format_cst_error)?;
    let indented = rewrite_line_indentation(source, &options, &events, &opaque_ranges);
    let indented_stream = LosslessTokenStream::from_source(&indented).map_err(FormatError::Lex)?;
    let (_, spaced_opaque_ranges) = collect_layout_metadata(&indented_stream)?;
    let spaced_cst = parse_lossless_cst(&indented, FileId(0)).map_err(format_cst_error)?;
    Ok(rewrite_token_spacing(
        &indented,
        spaced_cst.tokens(),
        &spaced_opaque_ranges,
    ))
}

/// Convert a lossless CST construction error into the formatter error contract.
fn format_cst_error(error: LosslessCstError) -> FormatError {
    match error {
        LosslessCstError::Lex(error) => FormatError::Lex(error),
        LosslessCstError::InvalidToken { span } => FormatError::InvalidToken { span },
        LosslessCstError::Parse { span, message } => FormatError::Parse { span, message },
        LosslessCstError::Diagnostics(mut diagnostics) => {
            let diagnostic = diagnostics.pop();
            diagnostic.map_or(
                FormatError::Parse {
                    span: Span::dummy(),
                    message: "parser diagnostics prevented formatting".to_owned(),
                },
                |diagnostic| FormatError::Parse {
                    span: diagnostic.primary,
                    message: diagnostic.code.to_owned(),
                },
            )
        }
        LosslessCstError::InvalidNodeSpan { node, span, .. } => FormatError::Parse {
            span,
            message: format!("invalid CST node span for node {}", node.0),
        },
        LosslessCstError::InvalidNodeId { index, node } => FormatError::Parse {
            span: Span::dummy(),
            message: format!("invalid CST node id {} at arena index {}", node.0, index),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BraceEvent {
    offset: usize,
    kind: TokenKind,
}

/// Collect brace events and source ranges whose internal layout must not change.
fn collect_layout_metadata(
    stream: &LosslessTokenStream,
) -> Result<(Vec<BraceEvent>, Vec<Span>), FormatError> {
    let mut events = Vec::new();
    let mut opaque_ranges = Vec::new();
    let mut interpolation_start = None;

    for token in stream.tokens() {
        if token.kind == TokenKind::Error {
            return Err(FormatError::InvalidToken { span: token.span });
        }
        match token.kind {
            TokenKind::StringLiteral => opaque_ranges.push(token.span),
            TokenKind::StringStart => interpolation_start = Some(token.span.start),
            TokenKind::StringEnd => {
                if let Some(start) = interpolation_start.take() {
                    opaque_ranges.push(Span::new(start, token.span.end));
                }
            }
            TokenKind::LBrace | TokenKind::RBrace if interpolation_start.is_none() => {
                events.push(BraceEvent {
                    offset: token.span.start,
                    kind: token.kind,
                });
            }
            _ => {}
        }
        collect_block_comments(stream, token.leading_trivia, &mut opaque_ranges);
    }
    if let Some(start) = interpolation_start {
        opaque_ranges.push(Span::new(start, stream.source().len()));
    }

    validate_brace_balance(&events)?;
    Ok((events, opaque_ranges))
}

/// Preserve multiline block-comment bodies exactly like string bodies.
fn collect_block_comments(stream: &LosslessTokenStream, trivia: Span, ranges: &mut Vec<Span>) {
    let Some(text) = stream.slice(trivia) else {
        return;
    };
    let mut cursor = 0;
    while cursor < text.len() {
        let Some(character) = text[cursor..].chars().next() else {
            break;
        };
        if character == '/' && text[cursor + 1..].starts_with('/') {
            cursor += 2;
            while cursor < text.len() && !text[cursor..].starts_with('\n') {
                cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
            }
            continue;
        }
        if character == '/' && text[cursor + 1..].starts_with('*') {
            let start = cursor;
            cursor += 2;
            while cursor < text.len() {
                if text[cursor..].starts_with("*/") {
                    cursor += 2;
                    break;
                }
                cursor += text[cursor..].chars().next().map_or(1, char::len_utf8);
            }
            ranges.push(Span::new(trivia.start + start, trivia.start + cursor));
            continue;
        }
        cursor += character.len_utf8();
    }
}

/// Refuse to rewrite source with lexically unbalanced code braces.
fn validate_brace_balance(events: &[BraceEvent]) -> Result<(), FormatError> {
    let mut depth = 0usize;
    for event in events {
        match event.kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => {
                if depth == 0 {
                    return Err(FormatError::UnbalancedBraces {
                        span: Span::new(event.offset, event.offset + 1),
                    });
                }
                depth -= 1;
            }
            _ => unreachable!("layout events only contain braces"),
        }
    }
    if depth == 0 {
        return Ok(());
    }
    let offset = events.last().map_or(0, |event| event.offset);
    Err(FormatError::UnbalancedBraces {
        span: Span::new(offset, offset + 1),
    })
}

/// Rewrite only horizontal leading whitespace on lines outside opaque ranges.
fn rewrite_line_indentation(
    source: &str,
    options: &FormatOptions,
    events: &[BraceEvent],
    opaque_ranges: &[Span],
) -> String {
    let mut output = String::with_capacity(source.len());
    let mut event_index = 0;
    let mut depth = 0usize;
    let mut line_start = 0;

    for segment in source.split_inclusive('\n') {
        let segment_end = line_start + segment.len();
        let newline_len =
            usize::from(segment.ends_with('\n')) + usize::from(segment.ends_with("\r\n"));
        let content_end = segment_end - newline_len;
        let content = &source[line_start..content_end];
        let leading_len = content
            .bytes()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count();
        let code_start = line_start + leading_len;
        let is_blank = code_start == content_end;
        let is_inside_opaque = line_start > 0
            && opaque_ranges
                .iter()
                .any(|range| line_start >= range.start && line_start < range.end);

        while event_index < events.len() && events[event_index].offset < line_start {
            apply_event(events[event_index], &mut depth);
            event_index += 1;
        }
        let line_depth = depth;
        let first_event_is_close = event_index < events.len()
            && events[event_index].offset >= code_start
            && events[event_index].offset < content_end
            && events[event_index].kind == TokenKind::RBrace;
        let target_depth = line_depth.saturating_sub(usize::from(first_event_is_close));

        if is_blank || is_inside_opaque {
            output.push_str(content);
        } else {
            write_indentation(&mut output, target_depth, options);
            output.push_str(&source[code_start..content_end]);
        }
        output.push_str(&source[content_end..segment_end]);

        while event_index < events.len() && events[event_index].offset < content_end {
            apply_event(events[event_index], &mut depth);
            event_index += 1;
        }
        line_start = segment_end;
    }

    output
}

/// Write one canonical indentation prefix without changing source content.
fn write_indentation(output: &mut String, depth: usize, options: &FormatOptions) {
    let (character, count) = if options.use_tabs {
        ('\t', depth)
    } else {
        (' ', depth * options.indent_width)
    };
    output.extend(std::iter::repeat_n(character, count));
}

/// Apply one brace event to the running indentation depth.
fn apply_event(event: BraceEvent, depth: &mut usize) {
    match event.kind {
        TokenKind::LBrace => *depth += 1,
        TokenKind::RBrace => *depth = depth.saturating_sub(1),
        _ => unreachable!("layout events only contain braces"),
    }
}

#[cfg(test)]
mod tests {
    use super::{FormatError, format_source, format_source_with_options};

    #[test]
    fn formats_nested_code_indentation_without_rewriting_content() {
        let source = "main = () -> i64 {\nanswer = 40 + 2\nanswer\n}\n";
        let formatted = format_source(source).expect("balanced source must format");
        assert_eq!(
            formatted,
            "main = () -> i64 {\n  answer = 40 + 2\n  answer\n}\n"
        );
        assert_eq!(
            format_source(&formatted).expect("formatted source must be idempotent"),
            formatted
        );
    }

    #[test]
    fn canonicalizes_spacing_and_is_idempotent() {
        let source = "main=()->i64{\nanswer=40+2\nprint(answer,a=1)\n}\n";
        let formatted = format_source(source).expect("valid source must format");
        assert_eq!(
            formatted,
            "main = () -> i64 {\n  answer = 40 + 2\n  print(answer, a = 1)\n}\n"
        );
        assert_eq!(
            format_source(&formatted).expect("formatted source must be idempotent"),
            formatted
        );
    }

    #[test]
    fn distinguishes_import_paths_from_division() {
        let source = "{main}=@scope/pkg\n{other}=./foo\nmain=()->i64{answer=-40/2}\n";
        let formatted = format_source(source).expect("valid source must format");
        assert_eq!(
            formatted,
            "{ main } = @scope/pkg\n{ other } = ./foo\nmain = () -> i64 { answer = -40 / 2 }\n"
        );
    }

    #[test]
    fn formats_quantifiers_without_changing_string_contents() {
        let source = "main=()->bool{\nall(x>1,y<2)\nmessage=`a+b={x}`\n}\n";
        let formatted = format_source(source).expect("valid source must format");
        assert_eq!(
            formatted,
            "main = () -> bool {\n  all (x > 1, y < 2)\n  message = `a+b={x}`\n}\n"
        );
    }

    #[test]
    fn canonicalizes_declarations_access_calls_and_regex_safely() {
        let source = "#[type=Manifest]Config=struct{name:string=`x` count:i64=0}\r\nmain=()->bool{\r\nvalue=foo[0].bar+arr[1]\r\ngroup=(40+2)\r\ncheck=not(value)\r\nall(value>0)\r\nmatched=value { matches /a + b/i { true } { false } } // keep  +  \r\n}\r\n";
        let formatted = format_source(source).expect("valid source must format");
        assert_eq!(
            formatted,
            "#[type = Manifest] Config = struct { name: string = `x` count: i64 = 0 }\r\nmain = () -> bool {\r\n  value = foo[0].bar + arr[1]\r\n  group = (40 + 2)\r\n  check = not (value)\r\n  all (value > 0)\r\n  matched = value { matches /a + b/i { true } { false } } // keep  +  \r\n}\r\n"
        );
        assert_eq!(
            format_source(&formatted).expect("formatted source must be idempotent"),
            formatted
        );
    }

    #[test]
    fn preserves_comments_strings_and_crlf_line_endings() {
        let source = "main = () -> i64 {\r\nanswer = `text {not-code}` /* keep */\r\n}\r\n";
        let formatted = format_source(source).expect("balanced source must format");
        assert_eq!(
            formatted,
            "main = () -> i64 {\r\n  answer = `text {not-code}` /* keep */\r\n}\r\n"
        );
    }

    #[test]
    fn leaves_comment_separated_operator_gap_untouched() {
        let source = "main=()->i64{\nanswer=40/*keep*/+2\n}\n";
        let formatted = format_source(source).expect("valid source must format");
        assert_eq!(
            formatted,
            "main = () -> i64 {\n  answer = 40/*keep*/+ 2\n}\n"
        );
    }

    #[test]
    fn leaves_line_comment_text_unchanged() {
        let source = "main = () -> i64 {\nanswer = 42 // /* not a block comment\n}\n";
        let formatted = format_source(source).expect("balanced source must format");
        assert_eq!(
            formatted,
            "main = () -> i64 {\n  answer = 42 // /* not a block comment\n}\n"
        );
    }

    #[test]
    fn formats_with_tabs_when_requested() {
        let source = "main = () -> i64 {\nanswer = 42\n}\n";
        let formatted = format_source_with_options(
            source,
            super::FormatOptions {
                indent_width: 4,
                use_tabs: true,
            },
        )
        .expect("balanced source must format");
        assert_eq!(formatted, "main = () -> i64 {\n\tanswer = 42\n}\n");
    }

    #[test]
    fn rejects_invalid_tokens_and_unbalanced_braces() {
        assert!(matches!(
            format_source("main = () -> i64 { $$$"),
            Err(FormatError::InvalidToken { .. })
        ));
        assert!(matches!(
            format_source("main = () -> i64 { 42"),
            Err(FormatError::UnbalancedBraces { .. })
        ));
        assert!(matches!(
            format_source("main = ("),
            Err(FormatError::Parse { .. })
        ));
    }

    #[test]
    fn rejects_zero_indent_width() {
        assert!(matches!(
            format_source_with_options(
                "main = () -> i64 { 42 }",
                super::FormatOptions {
                    indent_width: 0,
                    use_tabs: false,
                }
            ),
            Err(FormatError::InvalidIndentWidth { width: 0 })
        ));
    }
}
