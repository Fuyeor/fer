// syntax/src/lib.rs

pub mod cst;
pub mod formatter;
pub mod grammar;
pub mod lex;
pub mod lossless;
pub mod lossless_cst;
pub mod parse;

// Re-export commonly used types
pub use cst::{CstNode, NodeId};
pub use formatter::{FormatError, FormatOptions, format_source, format_source_with_options};
pub use lex::{Lexer, Token, decode_string_literal, normalize_multiline_string};
pub use lossless::{LosslessLexError, LosslessToken, LosslessTokenStream};
pub use lossless_cst::{
    LosslessCst, LosslessCstError, TokenRange, associate_lossless_tokens, parse_lossless_cst,
};
pub use parse::Parser;
