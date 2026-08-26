// syntax/src/lex/string.rs

use crate::grammar::TokenKind;
use infra::Span;

use super::{Lexer, Token};

#[derive(Clone, Copy)]
pub(super) enum StringMode {
    Text,
    Expression { brace_depth: usize },
}

impl<'a> Lexer<'a> {
    /// Scan a backtick string and choose the plain or interpolated token path.
    pub(super) fn lex_string_literal(&mut self) -> Token {
        let open_pos = self.pos;
        self.pos += 1; // consume opening backtick
        let content_start = self.pos;
        if self.string_has_interpolation(content_start) {
            self.string_mode = Some(StringMode::Text);
            return self.make_token(TokenKind::StringStart, open_pos, self.pos);
        }

        // Scan until closing backtick, handling escapes.
        while self.pos < self.source.len() {
            let c = self.current_char();
            if c == '`' {
                let end_pos = self.pos + 1;
                let text = self.collect_text_and_crop_indent(content_start, self.pos);
                self.pos += 1; // consume closing backtick
                return Token {
                    kind: TokenKind::StringLiteral,
                    span: Span::new(open_pos, end_pos),
                    symbol: Some(self.interner.intern(&text)),
                };
            } else if c == '\\' {
                self.skip_escaped_character();
            } else {
                self.pos += c.len_utf8();
            }
        }
        // Unterminated string
        self.error_token("unterminated string literal")
    }

    /// Produce text, expression delimiters, and the closing string token.
    pub(super) fn lex_string_text(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.source.len() {
            match self.current_char() {
                '`' | '{' => break,
                '\\' => self.skip_escaped_character(),
                _ => self.pos += self.current_char().len_utf8(),
            }
        }
        if self.pos > start {
            let raw = &self.source[start..self.pos];
            return Token {
                kind: TokenKind::StringPart,
                span: Span::new(start, self.pos),
                symbol: Some(self.interner.intern(&unescape(raw))),
            };
        }
        if self.is_eof() {
            self.string_mode = None;
            return self.error_token("unterminated interpolated string");
        }
        let delimiter = self.current_char();
        self.pos += delimiter.len_utf8();
        if delimiter == '`' {
            self.string_mode = None;
            return self.make_token(TokenKind::StringEnd, start, self.pos);
        }
        self.string_mode = Some(StringMode::Expression { brace_depth: 0 });
        self.make_token(TokenKind::ExprStart, start, self.pos)
    }

    /// Switch between normal expression lexing and the outer interpolation text.
    pub(super) fn lex_string_expression(&mut self) -> Token {
        self.skip_whitespace_and_comments();
        if self.is_eof() {
            self.string_mode = None;
            return self.error_token("unterminated interpolated expression");
        }
        let start = self.pos;
        let Some(StringMode::Expression { brace_depth }) = self.string_mode else {
            unreachable!("string expression mode must be active");
        };
        match self.current_char() {
            '{' => {
                self.pos += 1;
                self.string_mode = Some(StringMode::Expression {
                    brace_depth: brace_depth + 1,
                });
                self.make_token(TokenKind::LBrace, start, self.pos)
            }
            '}' if brace_depth == 0 => {
                self.pos += 1;
                self.string_mode = Some(StringMode::Text);
                self.make_token(TokenKind::ExprEnd, start, self.pos)
            }
            '}' => {
                self.pos += 1;
                self.string_mode = Some(StringMode::Expression {
                    brace_depth: brace_depth - 1,
                });
                self.make_token(TokenKind::RBrace, start, self.pos)
            }
            _ => self.lex_normal(),
        }
    }

    fn string_has_interpolation(&self, mut pos: usize) -> bool {
        while pos < self.source.len() {
            match self.source[pos..].chars().next().unwrap_or('\0') {
                '`' => return false,
                '{' => return true,
                '\\' => {
                    pos += 1;
                    if pos < self.source.len() {
                        pos += self.source[pos..].chars().next().map_or(0, char::len_utf8);
                    }
                }
                character => pos += character.len_utf8(),
            }
        }
        false
    }

    fn collect_text_and_crop_indent(&self, text_start: usize, end_pos: usize) -> String {
        normalize_multiline_string(&self.source[text_start..end_pos])
    }

    fn skip_escaped_character(&mut self) {
        self.pos += 1;
        if !self.is_eof() {
            self.pos += self.current_char().len_utf8();
        }
    }
}

/// Decode the content of a backtick-delimited string literal.
pub fn decode_string_literal(source: &str) -> String {
    let content = source
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(source);
    normalize_multiline_string(content)
}

/// Normalize indentation and escapes in a string's already-delimited content.
pub fn normalize_multiline_string(raw: &str) -> String {
    let mut lines: Vec<&str> = raw.split('\n').collect();
    if lines
        .first()
        .is_some_and(|line| line.is_empty() || line.trim().is_empty())
        && lines.first() == Some(&"")
    {
        lines.remove(0);
    }
    if lines.len() > 1 && lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let min_indent = lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let trimmed = lines
        .iter()
        .map(|line| line.get(min_indent..).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    unescape(&trimmed)
}

fn unescape(s: &str) -> String {
    // Decode only escapes defined by the current Fer string draft.
    let mut result = String::with_capacity(s.len());
    let mut characters = s.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match characters.next() {
            Some('`') => result.push('`'),
            Some('{') => result.push('{'),
            Some('}') => result.push('}'),
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some('\n') => skip_continuation_indent(&mut characters),
            Some('\r') => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                skip_continuation_indent(&mut characters);
            }
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

fn skip_continuation_indent(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while matches!(characters.peek(), Some(' ' | '\t')) {
        characters.next();
    }
}
