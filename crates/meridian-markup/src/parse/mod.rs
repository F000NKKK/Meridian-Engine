//! Turning `.meui` text into a syntax tree: the parser, its errors and
//! source positions.

mod error;
mod line_index;
mod parser;
mod span;

pub use error::{ParseError, ParseErrorKind};
pub use parser::parse;
pub use span::Span;
