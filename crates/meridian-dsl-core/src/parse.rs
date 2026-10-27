//! Entry point: source text to an element tree.

use crate::Element;
use crate::ParseError;
use crate::Parser;

/// Parses a complete DSL document into its single root [`Element`](crate::Element).
/// Leading/trailing whitespace around the root is ignored; anything
/// else outside the root tag (a second root element, stray text) is a
/// [`ParseError`](crate::ParseError).
pub fn parse(source: &str) -> Result<Element, ParseError> {
    let mut parser = Parser {
        bytes: source.as_bytes(),
        pos: 0,
    };
    parser.skip_whitespace();
    let root = parser.parse_element()?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(parser.error("expected end of document after root element"));
    }
    Ok(root)
}
