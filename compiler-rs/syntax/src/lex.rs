// syntax/src/lex.rs

use crate::grammar::{TokenKind, keyword_token};
use infra::{Interner, Span, Symbol};

mod string;
use string::StringMode;
pub use string::{decode_string_literal, normalize_multiline_string};

#[cfg(test)]
mod lexer_tests;

#[cfg(test)]
mod string_tests;

/// A single token produced by the lexer.
#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// Interned text for identifiers and string parts, `None` for other tokens.
    pub symbol: Option<Symbol>,
}

/// Lexer state machine.
pub struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    interner: &'a mut Interner,
    /// Track the start of the current token for span calculation.
    token_start: usize,
    /// true after 'matches' keyword
    regex_mode: bool,
    string_mode: Option<StringMode>,
}

/// A saved lexer position that can be restored later.
#[derive(Clone)]
pub struct LexerCheckpoint {
    pos: usize,
    token_start: usize,
    regex_mode: bool,
    string_mode: Option<StringMode>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, interner: &'a mut Interner) -> Self {
        Self {
            source,
            pos: 0,
            interner,
            token_start: 0,
            regex_mode: false,
            string_mode: None,
        }
    }

    pub fn set_regex_mode(&mut self, mode: bool) {
        self.regex_mode = mode;
    }

    /// Return the source text covered by a valid token span.
    pub(crate) fn source_text(&self, span: Span) -> Option<&str> {
        self.source.get(span.start..span.end)
    }

    /// Advance to the next token.
    /// Returns `Token { kind: Eof, .. }` after the source is exhausted.
    pub fn next_token(&mut self) -> Token {
        match self.string_mode {
            Some(StringMode::Text) => self.lex_string_text(),
            Some(StringMode::Expression { .. }) => self.lex_string_expression(),
            None => self.lex_normal(),
        }
    }

    pub fn checkpoint(&self) -> LexerCheckpoint {
        LexerCheckpoint {
            pos: self.pos,
            token_start: self.token_start,
            regex_mode: self.regex_mode,
            string_mode: self.string_mode,
        }
    }

    pub fn restore(&mut self, ck: LexerCheckpoint) {
        self.pos = ck.pos;
        self.token_start = ck.token_start;
        self.regex_mode = ck.regex_mode;
        self.string_mode = ck.string_mode;
    }

    pub(crate) fn symbol_text(&self, symbol: Symbol) -> Option<&str> {
        self.interner.lookup(symbol)
    }

    // -------------------- Normal mode --------------------
    fn lex_normal(&mut self) -> Token {
        loop {
            self.skip_whitespace_and_comments();
            if self.is_eof() {
                return self.make_token(TokenKind::Eof, self.pos, self.pos);
            }
            self.token_start = self.pos;
            let regex_mode = self.regex_mode;
            self.regex_mode = false;
            let c = self.current_char();
            match c {
                '`' => return self.lex_string_literal(),
                '0'..='9' => return self.lex_number(),
                'a'..='z' | 'A'..='Z' | '_' => return self.lex_identifier(),
                '=' => return self.single_char_token(TokenKind::Eq),
                '<' => {
                    self.pos += 1;
                    if self.current_char() == '=' {
                        self.pos += 1;
                        return self.make_token(TokenKind::LtEq, self.token_start, self.pos);
                    }
                    return self.make_token(TokenKind::Lt, self.token_start, self.pos);
                }
                '>' => {
                    self.pos += 1;
                    if self.current_char() == '=' {
                        self.pos += 1;
                        return self.make_token(TokenKind::GtEq, self.token_start, self.pos);
                    }
                    return self.make_token(TokenKind::Gt, self.token_start, self.pos);
                }
                '-' => {
                    self.pos += 1;
                    if self.current_char() == '>' {
                        self.pos += 1;
                        return self.make_token(TokenKind::Arrow, self.token_start, self.pos);
                    }
                    return self.make_token(TokenKind::Minus, self.token_start, self.pos);
                }
                '+' => return self.single_char_token(TokenKind::Plus),
                '*' => return self.single_char_token(TokenKind::Star),
                '/' => {
                    if regex_mode {
                        return self.scan_regex_token();
                    }
                    // Check comments
                    if self.peek_char() == Some('/') {
                        self.pos += 2;
                        while self.pos < self.source.len() && self.current_char() != '\n' {
                            self.advance_char();
                        }
                        continue; // line comment
                    } else if self.peek_char() == Some('*') {
                        self.pos += 2;
                        while self.pos < self.source.len() {
                            if self.current_char() == '*' && self.peek_char() == Some('/') {
                                self.pos += 2;
                                break;
                            }
                            self.advance_char();
                        }
                        continue; // block comment
                    }
                    return self.single_char_token(TokenKind::Slash);
                }
                '%' => return self.single_char_token(TokenKind::Percent),
                '(' => return self.single_char_token(TokenKind::LParen),
                ')' => return self.single_char_token(TokenKind::RParen),
                '{' => return self.single_char_token(TokenKind::LBrace),
                '}' => return self.single_char_token(TokenKind::RBrace),
                '[' => return self.single_char_token(TokenKind::LBracket),
                ']' => return self.single_char_token(TokenKind::RBracket),
                ',' => return self.single_char_token(TokenKind::Comma),
                '.' => return self.single_char_token(TokenKind::Dot),
                ':' => return self.single_char_token(TokenKind::Colon),
                '@' => return self.single_char_token(TokenKind::At),
                '#' => return self.single_char_token(TokenKind::Hash),
                _ => return self.error_token("unexpected character"),
            }
        }
    }

    // -------------------- Helpers --------------------
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.source.len() {
            let c = self.current_char();
            match c {
                ' ' | '\t' | '\n' | '\r' => {
                    self.pos += 1;
                }
                '/' => {
                    if self.peek_char() == Some('/') {
                        // Line comment: skip until newline.
                        self.pos += 2;
                        while self.pos < self.source.len() && self.current_char() != '\n' {
                            self.advance_char();
                        }
                    } else if self.peek_char() == Some('*') {
                        // Block comment: skip until */
                        self.pos += 2;
                        while self.pos < self.source.len() {
                            if self.current_char() == '*' && self.peek_char() == Some('/') {
                                self.pos += 2;
                                break;
                            }
                            self.advance_char();
                        }
                    } else {
                        break; // division operator, not comment
                    }
                }
                _ => break,
            }
        }
    }

    fn scan_regex_token(&mut self) -> Token {
        let start = self.pos;
        self.advance_char(); // consume opening '/'
        while self.pos < self.source.len() {
            let c = self.current_char();
            if c == '\\' {
                self.pos += c.len_utf8();
                if !self.is_eof() {
                    self.advance_char();
                }
            } else if c == '/' {
                self.advance_char(); // closing '/'
                // Scan flags
                while self.pos < self.source.len() && self.current_char().is_ascii_alphabetic() {
                    self.advance_char();
                }
                let span = Span::new(start, self.pos);
                return Token {
                    kind: TokenKind::RegexLiteral,
                    span,
                    symbol: None,
                };
            } else {
                self.advance_char();
            }
        }
        // Unterminated regex
        let span = Span::new(start, self.pos);
        Token {
            kind: TokenKind::Error,
            span,
            symbol: None,
        }
    }

    fn lex_number(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.source.len() && self.current_char().is_ascii_digit() {
            self.pos += 1;
        }
        let is_float = self.pos < self.source.len()
            && self.current_char() == '.'
            && self.peek_next_char().is_some_and(|c| c.is_ascii_digit());
        if is_float {
            self.pos += 1; // skip '.'
            while self.pos < self.source.len() && self.current_char().is_ascii_digit() {
                self.pos += 1;
            }
            self.make_token(TokenKind::FloatLiteral, start, self.pos)
        } else {
            self.make_token(TokenKind::IntLiteral, start, self.pos)
        }
    }

    fn lex_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.source.len() {
            let c = self.current_char();
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let word = &self.source[start..self.pos];
        if let Some(kind) = keyword_token(word) {
            if kind == TokenKind::Matches {
                self.regex_mode = true; // next token will be a regex
            }
            self.make_token(kind, start, self.pos)
        } else {
            let sym = self.interner.intern(word);
            Token {
                kind: TokenKind::Identifier,
                span: Span::new(start, self.pos),
                symbol: Some(sym),
            }
        }
    }

    fn single_char_token(&mut self, kind: TokenKind) -> Token {
        self.pos += 1;
        if kind == TokenKind::Eq {
            self.regex_mode = true;
        }
        self.make_token(kind, self.token_start, self.pos)
    }

    fn make_token(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, end),
            symbol: None,
        }
    }

    fn error_token(&mut self, _msg: &str) -> Token {
        // Produce an Error token spanning the current character.
        let start = self.pos;
        if !self.is_eof() {
            self.pos += self.current_char().len_utf8();
        }
        // We could also store the error message, but for now it's just a token.
        Token {
            kind: TokenKind::Error,
            span: Span::new(start, self.pos),
            symbol: None,
        }
    }

    fn current_char(&self) -> char {
        self.source[self.pos..].chars().next().unwrap_or('\0')
    }

    /// Advance the cursor by exactly one UTF-8 scalar value.
    fn advance_char(&mut self) {
        if !self.is_eof() {
            self.pos += self.current_char().len_utf8();
        }
    }

    fn peek_char(&self) -> Option<char> {
        // Look at the character immediately after self.pos.
        if self.pos + 1 < self.source.len() {
            self.source[self.pos..].chars().nth(1)
        } else {
            None
        }
    }

    fn peek_next_char(&self) -> Option<char> {
        // Returns the character immediately after self.pos.
        if self.pos + 1 < self.source.len() {
            self.source[self.pos..].chars().nth(1)
        } else {
            None
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }
}
