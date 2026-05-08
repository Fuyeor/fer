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
    // /
    Slash,
    // @
    At,
    // ,
    Comma,

    // keywords
    Enum,
    Struct,
    Match,

    Greater,   // >
    Less,      // <
    GreaterEq, // >=
    LessEq,    // <=
    DoubleEq,  // ==
    NotEq,     // !=

    // End of file
    Eof,
}
