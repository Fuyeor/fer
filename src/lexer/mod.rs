// src/lexer/mod.rs
pub mod token;

use crate::lexer::token::Token;

pub struct Lexer<'a> {
    input: std::str::Chars<'a>,
    current_char: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.chars(),
            current_char: None,
        };
        lexer.advance();
        // Skip leading whitespace/newlines to find the mandatory header
        lexer.skip_whitespace();
        lexer
    }

    fn advance(&mut self) {
        self.current_char = self.input.next();
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let Some(char) = self.current_char else {
            return Token::Eof;
        };

        match char {
            '/' => {
                self.advance(); // consume first '/'
                if self.current_char == Some('/') {
                    self.advance(); // consume second '/'
                    if self.current_char == Some('/') {
                        self.advance(); // consume third '/'
                        self.skip_whitespace();

                        // Rule: Must start with '@'
                        if self.current_char == Some('@') {
                            let mut path = String::new();
                            while let Some(ch) = self.current_char {
                                if ch == '\n' || ch == '\r' {
                                    break;
                                }
                                path.push(ch);
                                self.advance();
                            }
                            return Token::PathComment(path.trim().to_string());
                        }
                        panic!("Header comment must start with '@' after '///'");
                    } else {
                        // It's a normal comment //, skip it and get next token
                        while let Some(ch) = self.current_char {
                            if ch == '\n' || ch == '\r' {
                                break;
                            }
                            self.advance();
                        }
                        return self.next_token();
                    }
                }
                Token::Slash // Just a division operator /
            }
            '=' => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::DoubleEq
                } else {
                    Token::Equals
                }
            }
            '(' => {
                self.advance();
                Token::LParen
            }
            ')' => {
                self.advance();
                Token::RParen
            }
            '{' => {
                self.advance();
                Token::LBrace
            }
            '}' => {
                self.advance();
                Token::RBrace
            }
            '.' => {
                self.advance();
                Token::Dot
            }
            '@' => {
                self.advance();
                Token::At
            }
            ',' => {
                self.advance();
                Token::Comma
            }
            '[' => {
                self.advance();
                Token::LBracket
            }
            ']' => {
                self.advance();
                Token::RBracket
            }
            // Parse Fer's signature backtick strings: `Hello World`
            '`' => {
                self.advance(); // Skip opening backtick
                let mut string_val = String::new();
                while let Some(ch) = self.current_char {
                    if ch == '`' {
                        self.advance(); // Skip closing backtick
                        break;
                    }
                    string_val.push(ch);
                    self.advance();
                }
                Token::StringLit(string_val)
            }

            // operators
            '>' => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::GreaterEq
                } else {
                    Token::Greater
                }
            }
            '<' => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::LessEq
                } else {
                    Token::Less
                }
            }
            '!' => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::NotEq
                } else {
                    panic!("Unexpected '!' without '='")
                }
            }

            // Handle Numbers
            char if char.is_ascii_digit() => {
                let mut num_str = String::new();
                while let Some(char) = self.current_char {
                    if char.is_ascii_digit() {
                        num_str.push(char);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Token::Number(num_str.parse().unwrap())
            }

            // Parse Identifiers (kebab-case or PascalCase)
            char if char.is_alphabetic() => {
                let mut id = String::new();
                let is_first_upper = char.is_uppercase();

                while let Some(char) = self.current_char {
                    // Fer allows identifiers to contain letters, numbers, and hyphens (-)
                    if char.is_alphanumeric() || char == '-' {
                        if char == '_' {
                            panic!("Snake_case is forbidden! Use kebab-case.");
                        }
                        id.push(char);
                        self.advance();
                    } else {
                        break;
                    }
                }

                // Variables cannot be all uppercase
                if !is_first_upper && id.chars().all(|x| x.is_uppercase() && x != '-') {
                    panic!("ALL_CAPS identifiers are forbidden!");
                }

                // check if it's a keyword
                match id.as_str() {
                    "enum" => Token::Enum,
                    "struct" => Token::Struct,
                    "string" | "i64" | "i32" | "bool" => Token::Identifier(id), // 暂时保留，语义层再处理
                    _ => Token::Identifier(id),
                }
            }
            _ => panic!("Unexpected character: {}", char),
        }
    }
}
