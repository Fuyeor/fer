// syntax/src/lib.rs

pub mod cst;
pub mod grammar;
pub mod lex;
pub mod parse;

// Re-export commonly used types
pub use cst::{CstNode, NodeId};
pub use lex::{Lexer, Token, decode_string_literal, normalize_multiline_string};
pub use parse::Parser;
