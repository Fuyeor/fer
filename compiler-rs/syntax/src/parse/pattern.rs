// syntax/src/parse/pattern.rs

use super::{ParseError, Parser};
use crate::cst::NodeId;

impl<'a> Parser<'a> {
    /// Parse a pattern used in match arms or destructuring.
    pub fn parse_pattern(&mut self) -> Result<NodeId, ParseError> {
        todo!("pattern parsing not implemented")
    }
}
