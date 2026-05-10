// src/lexer/token.rs
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // /// @/examples/hello.fer
    PathComment(String),
    // message, print
    Identifier(String),
    // `Hello World`
    StringLit(String),
    Number(i64),
    // =
    Equals,
    // (
    LParen,
    // )
    RParen,

    // [
    LBracket,
    // ]
    RBracket,

    // {
    LBrace,
    // }
    RBrace,

    // .
    Dot,
    // @
    At,
    // ,
    Comma,

    // keywords
    Enum,
    Struct,

    Greater,   // >
    Less,      // <
    GreaterEq, // >=
    LessEq,    // <=
    DoubleEq,  // ==
    NotEq,     // !=

    // +
    Plus,
    // -
    Minus,
    // *
    Star,
    // /
    Slash,

    // End of file
    Eof,
}
