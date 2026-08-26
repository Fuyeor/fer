// compiler-rs/fon/src/formatter.rs

use fon_parser::{Token, TokenKind, Trivia, TriviaKind, parse};

/// Errors that prevent a safe FON formatting rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    Parse {
        start: usize,
        end: usize,
        code: String,
        message: String,
    },
    InvalidToken {
        start: usize,
        end: usize,
    },
}

/// Format one validated FON source while preserving source-owned opaque text.
pub fn format_source(source: &str) -> Result<String, FormatError> {
    let result = parse(source);
    if let Some(diagnostic) = result.diagnostics.first() {
        return Err(FormatError::Parse {
            start: diagnostic.span.start as usize,
            end: diagnostic.span.end as usize,
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
        });
    }

    let tokens = &result.document.cst.tokens;
    if let Some(token) = tokens.iter().find(|token| token.kind == TokenKind::Error) {
        return Err(FormatError::InvalidToken {
            start: token.span.start as usize,
            end: token.span.end as usize,
        });
    }
    let source_opaque_ranges = opaque_ranges(tokens, &result.document.cst.trivia);
    let indented = rewrite_indentation(source, tokens, &source_opaque_ranges);
    let reparsed = parse(&indented);
    if let Some(diagnostic) = reparsed.diagnostics.first() {
        return Err(FormatError::Parse {
            start: diagnostic.span.start as usize,
            end: diagnostic.span.end as usize,
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
        });
    }
    let reformatted_tokens = &reparsed.document.cst.tokens;
    let reformatted_ranges = opaque_ranges(reformatted_tokens, &reparsed.document.cst.trivia);
    Ok(rewrite_token_spacing(
        &indented,
        reformatted_tokens,
        &reformatted_ranges,
    ))
}

/// Collect string, regex, line-comment, and block-comment ranges that are opaque to layout.
fn opaque_ranges(tokens: &[Token], trivia: &[Trivia]) -> Vec<Range> {
    let mut ranges = tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::String | TokenKind::Regex))
        .map(|token| Range {
            start: token.span.start as usize,
            end: token.span.end as usize,
        })
        .collect::<Vec<_>>();
    ranges.extend(
        trivia
            .iter()
            .filter(|item| {
                matches!(
                    item.kind,
                    TriviaKind::LineComment | TriviaKind::BlockComment
                )
            })
            .map(|item| Range {
                start: item.span.start as usize,
                end: item.span.end as usize,
            }),
    );
    ranges
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Range {
    start: usize,
    end: usize,
}

/// Normalize only leading horizontal indentation outside opaque source ranges.
fn rewrite_indentation(source: &str, tokens: &[Token], opaque_ranges: &[Range]) -> String {
    let brace_events = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::LBrace | TokenKind::RBrace => {
                Some((token.span.start as usize, token.kind == TokenKind::LBrace))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut output = String::with_capacity(source.len());
    let mut depth = 0usize;
    let mut event_index = 0usize;
    let mut line_start = 0usize;

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
        let is_inside_opaque = opaque_ranges
            .iter()
            .any(|range| line_start >= range.start && line_start < range.end);

        while event_index < brace_events.len() && brace_events[event_index].0 < line_start {
            if brace_events[event_index].1 {
                depth += 1;
            } else {
                depth = depth.saturating_sub(1);
            }
            event_index += 1;
        }
        let line_depth = depth;
        let first_event_is_close = event_index < brace_events.len()
            && brace_events[event_index].0 >= code_start
            && brace_events[event_index].0 < content_end
            && !brace_events[event_index].1;
        let target_depth = line_depth.saturating_sub(usize::from(first_event_is_close));

        if is_blank || is_inside_opaque {
            output.push_str(content);
        } else {
            output.extend(std::iter::repeat_n(' ', target_depth * 2));
            output.push_str(&source[code_start..content_end]);
        }
        output.push_str(&source[content_end..segment_end]);

        while event_index < brace_events.len() && brace_events[event_index].0 < content_end {
            if brace_events[event_index].1 {
                depth += 1;
            } else {
                depth = depth.saturating_sub(1);
            }
            event_index += 1;
        }
        line_start = segment_end;
    }
    output
}

/// Rewrite only gaps made entirely of spaces or tabs between adjacent concrete tokens.
fn rewrite_token_spacing(source: &str, tokens: &[Token], opaque_ranges: &[Range]) -> String {
    let mut edits = Vec::new();
    for pair in tokens.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        let start = previous.span.end as usize;
        let end = next.span.start as usize;
        if start > end || !is_horizontal_gap(&source[start..end]) {
            continue;
        }
        if previous.kind == TokenKind::Newline || next.kind == TokenKind::Newline {
            continue;
        }
        if start < end
            && opaque_ranges
                .iter()
                .any(|range| start >= range.start && end <= range.end)
        {
            continue;
        }
        let previous_text = source
            .get(previous.span.start as usize..previous.span.end as usize)
            .unwrap_or_default();
        let next_text = source
            .get(next.span.start as usize..next.span.end as usize)
            .unwrap_or_default();
        let desired = desired_spacing(previous.kind, next.kind, previous_text, next_text);
        if desired != &source[start..end] {
            edits.push((start, end, desired.to_owned()));
        }
    }

    let mut output = source.to_owned();
    for (start, end, replacement) in edits.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    output
}

fn is_horizontal_gap(gap: &str) -> bool {
    gap.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn desired_spacing(
    previous_kind: TokenKind,
    next_kind: TokenKind,
    previous_text: &str,
    next_text: &str,
) -> &'static str {
    if next_kind == TokenKind::RBrace {
        return if previous_kind == TokenKind::LBrace {
            ""
        } else {
            " "
        };
    }
    if previous_kind == TokenKind::LBrace {
        return " ";
    }
    if matches!(next_kind, TokenKind::RBracket | TokenKind::RParen)
        || matches!(previous_kind, TokenKind::LBracket | TokenKind::LParen)
    {
        return "";
    }
    if previous_kind == TokenKind::Comma {
        return " ";
    }
    if previous_kind == TokenKind::Colon {
        return " ";
    }
    if matches!(
        previous_kind,
        TokenKind::Equals | TokenKind::Plus | TokenKind::Minus
    ) || matches!(
        next_kind,
        TokenKind::Equals | TokenKind::Plus | TokenKind::Minus
    ) || matches!(previous_kind, TokenKind::LessThan | TokenKind::GreaterThan)
        || matches!(next_kind, TokenKind::LessThan | TokenKind::GreaterThan)
    {
        return " ";
    }
    if previous_kind == TokenKind::Struct || previous_kind == TokenKind::Enum {
        return " ";
    }
    if previous_text == "#[" || next_text == "]" {
        return "";
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::{FormatError, format_source};

    #[test]
    fn formats_nested_objects_without_touching_opaque_text() {
        let source = "// leading\nname=`Fuyeor`\nconfig={\nmessage=`a+b={x}` // keep  +  \npattern=/a + b/i\n}\n";
        let formatted = format_source(source).expect("valid source must format");
        assert_eq!(
            formatted,
            "// leading\nname = `Fuyeor`\nconfig = {\n  message = `a+b={x}` // keep  +  \n  pattern = /a + b/i\n}\n"
        );
    }

    #[test]
    fn formats_single_line_assignments_without_indentation_changes() {
        assert_eq!(
            format_source("name=`Fuyeor`\n").expect("valid source must format"),
            "name = `Fuyeor`\n"
        );
        assert_eq!(
            format_source("config={value=1}\n").expect("valid source must format"),
            "config = { value = 1 }\n"
        );
    }

    #[test]
    fn rejects_invalid_source_before_rewriting() {
        assert!(matches!(
            format_source("config={\nvalue=\n"),
            Err(FormatError::Parse { .. })
        ));
    }
}
