//! The recursive-descent parser behind [`parse`].

use super::error::{ParseError, ParseErrorKind};
use super::line_index::LineIndex;
use crate::syntax::is_identifier_byte;
use crate::{Attribute, Element, MarkupValue, QualifiedName, QualifiedNameError};

/// Parses a complete `.mel` document into its single root [`Element`].
/// Leading/trailing whitespace around the root is ignored; anything
/// else outside the root tag (a second root element, stray text) is a
/// [`ParseError`].
pub fn parse(source: &str) -> Result<Element, ParseError> {
    let mut parser = Parser {
        bytes: source.as_bytes(),
        pos: 0,
        lines: LineIndex::new(source),
    };
    parser.skip_whitespace();
    let root = parser.parse_element(None)?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(parser.error(ParseErrorKind::TrailingContent));
    }
    Ok(root)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    lines: LineIndex<'a>,
}

impl Parser<'_> {
    fn error(&self, kind: ParseErrorKind) -> ParseError {
        self.error_at(kind, self.pos)
    }

    fn error_at(&self, kind: ParseErrorKind, offset: usize) -> ParseError {
        ParseError {
            kind,
            span: self.lines.span(offset),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b) if b.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), ParseError> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::Expected(char::from(expected))))
        }
    }

    /// A name: `[x:][Owner.]local`, see [`QualifiedName`].
    fn parse_name(&mut self) -> Result<QualifiedName, ParseError> {
        let start = self.pos;
        if !matches!(self.peek(), Some(b) if b.is_ascii_alphabetic() || b == b'_') {
            return Err(self.error(ParseErrorKind::ExpectedIdentifier));
        }
        while matches!(self.peek(), Some(b) if is_identifier_byte(b) || b == b':' || b == b'.') {
            self.pos += 1;
        }
        let text = String::from_utf8_lossy(&self.bytes[start..self.pos]);
        QualifiedName::parse(&text).map_err(|error| {
            let kind = match error {
                QualifiedNameError::UnknownPrefix(prefix) => ParseErrorKind::UnknownPrefix(prefix),
                QualifiedNameError::Malformed => ParseErrorKind::ExpectedIdentifier,
            };
            self.error_at(kind, start)
        })
    }

    /// A double-quoted attribute value. `\"` and `\\` are the only
    /// escapes — enough to write a literal quote in an attribute without
    /// needing a full escape grammar.
    fn parse_quoted_string(&mut self) -> Result<String, ParseError> {
        self.expect_byte(b'"')?;
        let mut value = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error(ParseErrorKind::UnterminatedString)),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(value);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    value.push(self.parse_escape()?);
                }
                Some(_) => value.push(self.next_char()?),
            }
        }
    }

    fn parse_escape(&mut self) -> Result<char, ParseError> {
        let escaped = match self.peek() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            _ => return Err(self.error(ParseErrorKind::InvalidEscape)),
        };
        self.pos += 1;
        Ok(escaped)
    }

    /// The next whole UTF-8 char; `pos` only ever advances by whole
    /// chars, so it always sits on a boundary.
    fn next_char(&mut self) -> Result<char, ParseError> {
        let rest = std::str::from_utf8(&self.bytes[self.pos..])
            .map_err(|_| self.error(ParseErrorKind::InvalidUtf8))?;
        let ch = rest
            .chars()
            .next()
            .ok_or_else(|| self.error(ParseErrorKind::UnterminatedString))?;
        self.pos += ch.len_utf8();
        Ok(ch)
    }

    /// The source offset of byte `unescaped` of a string literal whose
    /// content starts at `content_start`: each `\"` / `\\` escape is two
    /// source bytes for one value byte.
    fn source_offset(&self, content_start: usize, unescaped: usize) -> usize {
        let mut raw = content_start;
        for _ in 0..unescaped {
            raw += if self.bytes.get(raw) == Some(&b'\\') {
                2
            } else {
                1
            };
        }
        raw.min(self.bytes.len())
    }

    fn parse_attrs(&mut self) -> Result<Vec<Attribute>, ParseError> {
        let mut attrs: Vec<Attribute> = Vec::new();
        loop {
            self.skip_whitespace();
            if !matches!(self.peek(), Some(b) if b.is_ascii_alphabetic() || b == b'_') {
                return Ok(attrs);
            }
            let start = self.pos;
            let name = self.parse_name()?;
            if attrs.iter().any(|attr| attr.name == name) {
                let kind = ParseErrorKind::DuplicateAttribute(name.to_string());
                return Err(self.error_at(kind, start));
            }
            self.skip_whitespace();
            self.expect_byte(b'=')?;
            self.skip_whitespace();
            let value_start = self.pos;
            let text = self.parse_quoted_string()?;
            let value = MarkupValue::parse(&text).map_err(|error| {
                let attribute = name.to_string();
                let offset = self.source_offset(value_start + 1, error.offset);
                let kind = ParseErrorKind::Extension {
                    attribute,
                    error: error.kind,
                };
                self.error_at(kind, offset)
            })?;
            let span = self.lines.span(start);
            attrs.push(Attribute { name, value, span });
        }
    }

    fn parse_element(&mut self, parent: Option<&QualifiedName>) -> Result<Element, ParseError> {
        let start = self.pos;
        self.expect_byte(b'<')?;
        let name = self.parse_name()?;
        self.check_element_name(&name, parent, start)?;
        let attrs = self.parse_attrs()?;
        if name.owner().is_some() && !attrs.is_empty() {
            let kind = ParseErrorKind::PropertyElementAttributes(name.to_string());
            return Err(self.error_at(kind, start));
        }
        self.skip_whitespace();
        let mut element = Element {
            name,
            attrs,
            children: Vec::new(),
            span: self.lines.span(start),
        };
        if self.peek() == Some(b'/') {
            self.pos += 1;
            self.expect_byte(b'>')?;
            return Ok(element);
        }
        self.expect_byte(b'>')?;
        element.children = self.parse_children(&element.name)?;
        Ok(element)
    }

    /// Rejects `x:` elements and misplaced property elements.
    fn check_element_name(
        &self,
        name: &QualifiedName,
        parent: Option<&QualifiedName>,
        start: usize,
    ) -> Result<(), ParseError> {
        let kind = if name.prefix().is_some() {
            ParseErrorKind::DirectiveElement(name.to_string())
        } else {
            match (name.owner(), parent) {
                (None, _) => return Ok(()),
                (Some(_), None) => ParseErrorKind::PropertyElementAtRoot(name.to_string()),
                (Some(owner), Some(parent)) if parent.is_plain(owner) => return Ok(()),
                (Some(_), Some(parent)) => ParseErrorKind::PropertyElementOwner {
                    property: name.to_string(),
                    parent: parent.to_string(),
                },
            }
        };
        Err(self.error_at(kind, start))
    }

    /// Children up to and including the closing tag of `name`.
    fn parse_children(&mut self, name: &QualifiedName) -> Result<Vec<Element>, ParseError> {
        let mut children = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'<') && self.bytes.get(self.pos + 1) == Some(&b'/') {
                self.parse_closing_tag(name)?;
                return Ok(children);
            }
            if self.peek().is_none() {
                return Err(self.error(ParseErrorKind::UnterminatedElement(name.to_string())));
            }
            children.push(self.parse_element(Some(name))?);
        }
    }

    fn parse_closing_tag(&mut self, name: &QualifiedName) -> Result<(), ParseError> {
        self.pos += 2;
        let start = self.pos;
        let closing = self.parse_name()?;
        if &closing != name {
            let kind = ParseErrorKind::MismatchedClosingTag {
                expected: name.to_string(),
                found: closing.to_string(),
            };
            return Err(self.error_at(kind, start));
        }
        self.skip_whitespace();
        self.expect_byte(b'>')
    }
}

#[cfg(test)]
mod tests;
