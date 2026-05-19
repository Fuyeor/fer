// syntax/src/lex.rs

use crate::grammar::{TokenKind, keyword_token};
use infra::{Interner, Span, Symbol};

/// A single token produced by the lexer.
#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// Interned identifier for `Identifier` tokens, `None` otherwise.
    pub symbol: Option<Symbol>,
}

/// Lexer state machine.
pub struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    interner: &'a mut Interner,
    state: LexerState,
    /// Track the start of the current token for span calculation.
    token_start: usize,
    /// true after 'matches' keyword
    regex_mode: bool,
}

/// A saved lexer position that can be restored later.
#[derive(Clone)]
pub struct LexerCheckpoint {
    pos: usize,
    token_start: usize,
    state: LexerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerState {
    Normal,
    InString(StringState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StringState {
    /// Whether we've already emitted a StringStart token.
    started: bool,
    /// Byte offset where current text segment began.
    text_start: usize,
    /// The opening backtick position (for span).
    open_pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, interner: &'a mut Interner) -> Self {
        Self {
            source,
            pos: 0,
            interner,
            state: LexerState::Normal,
            token_start: 0,
            regex_mode: false,
        }
    }

    pub fn set_regex_mode(&mut self, mode: bool) {
        self.regex_mode = mode;
    }

    /// Skip whitespace but not comments.
    pub fn skip_whitespace(&mut self) {
        while self.pos < self.source.len() {
            match self.current_char() {
                ' ' | '\t' | '\n' | '\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Advance to the next token.
    /// Returns `Token { kind: Eof, .. }` after the source is exhausted.
    pub fn next_token(&mut self) -> Token {
        // Must branch without holding a mutable borrow on self.state.
        if matches!(self.state, LexerState::InString(_)) {
            self.lex_in_string()
        } else {
            self.lex_normal()
        }
    }

    pub fn checkpoint(&self) -> LexerCheckpoint {
        LexerCheckpoint {
            pos: self.pos,
            token_start: self.token_start,
            state: self.state,
        }
    }

    pub fn restore(&mut self, ck: LexerCheckpoint) {
        self.pos = ck.pos;
        self.token_start = ck.token_start;
        self.state = ck.state;
    }

    /// After parsing an interpolation expression, call this to switch the
    /// lexer back into string scanning mode.
    pub fn resume_string(&mut self) {
        let mut state = self.state;
        if let LexerState::InString(ref mut s) = state {
            s.started = true;
            s.text_start = self.pos;
        }
        self.state = state;
    }

    /// Returns true if the lexer is currently scanning a string.
    pub fn is_in_string(&self) -> bool {
        matches!(self.state, LexerState::InString(_))
    }

    // -------------------- Normal mode --------------------
    fn lex_normal(&mut self) -> Token {
        loop {
            self.skip_whitespace_and_comments();
            if self.is_eof() {
                return self.make_token(TokenKind::Eof, self.pos, self.pos);
            }
            self.token_start = self.pos;
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
                    if self.regex_mode {
                        self.regex_mode = false;
                        return self.scan_regex_token();
                    }
                    // Check comments
                    if self.peek_char() == Some('/') {
                        self.pos += 2;
                        while self.pos < self.source.len() && self.current_char() != '\n' {
                            self.pos += 1;
                        }
                        continue; // line comment
                    } else if self.peek_char() == Some('*') {
                        self.pos += 2;
                        while self.pos < self.source.len() {
                            if self.current_char() == '*' && self.peek_char() == Some('/') {
                                self.pos += 2;
                                break;
                            }
                            self.pos += 1;
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
                _ => {
                    self.pos += 1;
                    return self.error_token("unexpected character");
                }
            }
        }
    }

    fn lex_string_literal(&mut self) -> Token {
        let open_pos = self.pos;
        self.pos += 1; // consume opening backtick
        let content_start = self.pos;

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
                self.pos += 1; // skip '\'
                self.pos += 1; // skip escaped char (including backtick, n, t, etc.)
            } else {
                self.pos += 1;
            }
        }
        // Unterminated string
        self.error_token("unterminated string literal")
    }

    // -------------------- In-string mode --------------------
    fn lex_in_string(&mut self) -> Token {
        // Copy the current state so we can mutate a local copy without
        // holding a borrow on `self.state`.
        let mut state = self.state;
        if let LexerState::InString(ref mut s) = state {
            if !s.started {
                s.started = true;
                s.text_start = self.pos;
            }

            while self.pos < self.source.len() {
                let c = self.current_char();
                if c == '`' {
                    let text = self.collect_text_and_crop_indent(s.text_start, self.pos);
                    let end_pos = self.pos + 1;
                    self.pos += 1;
                    self.state = LexerState::Normal; // exit string mode
                    return Token {
                        kind: TokenKind::StringLiteral,
                        span: Span::new(s.open_pos, end_pos),
                        symbol: Some(self.interner.intern(&text)),
                    };
                } else if c == '\\' {
                    self.pos += 1; // skip '\'
                    self.pos += 1; // skip escaped char
                } else {
                    self.pos += 1;
                }
            }
            // EOF in string → error
            self.state = LexerState::Normal;
            self.error_token("unterminated string")
        } else {
            unreachable!()
        }
    }

    /// Collect the text between text_start and end_pos, removing leading
    /// indentation according to Fer's multi-line string rules.
    /// Also processes escape sequences.
    fn collect_text_and_crop_indent(&self, text_start: usize, end_pos: usize) -> String {
        let raw = &self.source[text_start..end_pos];
        // If the string starts with a newline (first char is '\n'), we treat as multiline.
        let mut lines: Vec<&str> = raw.split('\n').collect();
        // If the first line is empty (because string started with newline), remove it.
        if lines
            .first()
            .map_or(false, |l| l.is_empty() || l.trim().is_empty())
        {
            // The first line might be just whitespace before the newline? Actually the opening backtick
            // is followed by optional newline. If there's a newline immediately, raw starts with '\n'.
            // We'll split, and if the first element is empty, that means string started with newline.
            if !lines.is_empty() && lines[0].is_empty() {
                lines.remove(0);
            }
        }
        // Find minimum indentation among non-empty lines.
        let min_indent = lines
            .iter()
            .filter(|l| !l.is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        // Remove that many spaces from the start of each line.
        let trimmed: Vec<&str> = lines
            .iter()
            .map(|l| {
                if l.len() >= min_indent {
                    &l[min_indent..]
                } else {
                    *l
                }
            })
            .collect();
        let joined = trimmed.join("\n");
        // Process escape sequences (basic: backslash + anything).
        // We'll do a simple pass to replace common escapes.
        unescape(&joined)
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
                            self.pos += 1;
                        }
                    } else if self.peek_char() == Some('*') {
                        // Block comment: skip until */
                        self.pos += 2;
                        while self.pos < self.source.len() {
                            if self.current_char() == '*' && self.peek_char() == Some('/') {
                                self.pos += 2;
                                break;
                            }
                            self.pos += 1;
                        }
                    } else {
                        break; // division operator, not comment
                    }
                }
                _ => break,
            }
        }
    }

    /// Try to scan a regex literal starting from current position.
    /// Call this when the parser expects a regex (after `matches` keyword).
    pub fn scan_regex(&mut self) -> Option<Token> {
        if self.is_eof() || self.current_char() != '/' {
            return None;
        }
        let start = self.pos;
        self.pos += 1; // consume opening '/'
        // Scan pattern until unescaped '/'
        while self.pos < self.source.len() {
            let c = self.current_char();
            if c == '\\' {
                self.pos += 1; // skip escape char
                self.pos += 1; // skip escaped char
            } else if c == '/' {
                // End of pattern
                self.pos += 1; // consume closing '/'
                // Scan optional flags
                while self.pos < self.source.len() && self.current_char().is_ascii_alphabetic() {
                    self.pos += 1;
                }
                let span = Span::new(start, self.pos);
                return Some(Token {
                    kind: TokenKind::RegexLiteral,
                    span,
                    symbol: None,
                });
            } else {
                self.pos += 1;
            }
        }
        // Unterminated regex
        let span = Span::new(start, self.pos);
        self.pos = start; // reset position? we already consumed some chars, but it's an error
        None
    }

    fn scan_regex_token(&mut self) -> Token {
        let start = self.pos;
        self.pos += 1; // consume opening '/'
        while self.pos < self.source.len() {
            let c = self.current_char();
            if c == '\\' {
                self.pos += 2; // skip escaped char
            } else if c == '/' {
                self.pos += 1; // closing '/'
                // Scan flags
                while self.pos < self.source.len() && self.current_char().is_ascii_alphabetic() {
                    self.pos += 1;
                }
                let span = Span::new(start, self.pos);
                return Token {
                    kind: TokenKind::RegexLiteral,
                    span,
                    symbol: None,
                };
            } else {
                self.pos += 1;
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
            && self.peek_next_char().map_or(false, |c| c.is_ascii_digit());
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
        self.make_token(kind, self.token_start, self.pos)
    }

    fn make_token(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, end),
            symbol: None,
        }
    }

    fn error_token(&mut self, msg: &str) -> Token {
        // Produce an Error token spanning the current character.
        let start = self.pos;
        self.pos += 1; // skip the problematic char
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

fn unescape(s: &str) -> String {
    // Very basic escape processing for now.
    s.replace("\\`", "`")
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::TokenKind;

    fn lex_one(source: &str) -> Token {
        let mut interner = Interner::new();
        let mut lexer = Lexer::new(source, &mut interner);
        lexer.next_token()
    }

    #[test]
    fn eof_token() {
        let tok = lex_one("");
        assert_eq!(tok.kind, TokenKind::Eof);
    }

    #[test]
    fn integer_literal() {
        let tok = lex_one("42");
        assert_eq!(tok.kind, TokenKind::IntLiteral);
        assert_eq!(tok.span, Span::new(0, 2));
    }

    #[test]
    fn float_literal() {
        let tok = lex_one("3.14");
        assert_eq!(tok.kind, TokenKind::FloatLiteral);
    }

    #[test]
    fn identifier_kebab_case() {
        let tok = lex_one("my-var");
        assert_eq!(tok.kind, TokenKind::Identifier);
        assert!(tok.symbol.is_some());
    }

    #[test]
    fn identifier_with_underscore_allowed() {
        let tok = lex_one("my_var"); // will be checked in semantic phase
        assert_eq!(tok.kind, TokenKind::Identifier);
    }

    #[test]
    fn identifier_capital_allowed() {
        let tok = lex_one("StructName"); // valid for struct/enum names
        assert_eq!(tok.kind, TokenKind::Identifier);
    }

    #[test]
    fn keyword_enum() {
        let tok = lex_one("enum");
        assert_eq!(tok.kind, TokenKind::Enum);
        assert!(tok.symbol.is_none());
    }

    #[test]
    fn keyword_and() {
        let tok = lex_one("and");
        assert_eq!(tok.kind, TokenKind::And);
    }

    #[test]
    fn simple_string() {
        let mut interner = Interner::new();
        let mut lexer = Lexer::new("`hello`", &mut interner);
        let tok = lexer.next_token();
        assert_eq!(tok.kind, TokenKind::StringLiteral);
        if let Some(sym) = tok.symbol {
            assert_eq!(interner.lookup(sym), Some("hello"));
        } else {
            panic!("Expected symbol");
        }
    }

    #[test]
    fn operators() {
        assert_eq!(lex_one("+").kind, TokenKind::Plus);
        assert_eq!(lex_one("-").kind, TokenKind::Minus);
        assert_eq!(lex_one("*").kind, TokenKind::Star);
        assert_eq!(lex_one("/").kind, TokenKind::Slash);
        assert_eq!(lex_one("<").kind, TokenKind::Lt);
        assert_eq!(lex_one(">").kind, TokenKind::Gt);
        assert_eq!(lex_one("<=").kind, TokenKind::LtEq);
        assert_eq!(lex_one(">=").kind, TokenKind::GtEq);
        assert_eq!(lex_one("=").kind, TokenKind::Eq);
        assert_eq!(lex_one("->").kind, TokenKind::Arrow);
    }

    #[test]
    fn at_symbol() {
        assert_eq!(lex_one("@").kind, TokenKind::At);
    }

    #[test]
    fn single_quote_is_error() {
        let tok = lex_one("'");
        assert_eq!(tok.kind, TokenKind::Error);
    }
}
