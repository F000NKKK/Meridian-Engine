//! Generic tag-markup parser: text in, a tag/attribute/children tree
//! out. This crate is deliberately domain-blind — it has never heard of
//! `Mesh`, `RigidBody`, or any other scene concept. That knowledge lives
//! one layer up, in `meridian-sdk::dsl` (typed tag registration via
//! `#[dsl_tag]`) and further still in whatever custom tags a game
//! developer registers there. Splitting it this way is what makes the
//! DSL *extensible* rather than a fixed schema (see
//! `docs/adr/*dsl*.md` for the decision this crate implements): the
//! parser never needs to change when a new tag is added, because it
//! never knew about tags in the first place, only about `<Name attr="value">`
//! syntax.
//!
//! Syntax, deliberately small (XAML-flavored, not a XAML implementation):
//! `<Tag attr="value" other="123"> ...children... </Tag>` or the
//! self-closing `<Tag attr="value" />`. One root element per document.
//! No namespaces, no CDATA, no processing instructions, no text nodes
//! between elements (a DSL document describes structure, not prose) —
//! anything beyond that is deliberately out of scope until a real
//! caller needs it.

use std::fmt;

/// One parsed element: its tag name, its attributes (in source order,
/// duplicates rejected at parse time rather than silently
/// last-write-wins — a duplicate attribute is almost always a typo, not
/// intent), and its children in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
}

impl Element {
    /// The first attribute value matching `name`, if any — attributes
    /// are stored as a `Vec` (not a `HashMap`) so source order survives
    /// for tools that want to echo it back (formatters, error messages),
    /// but lookups still want to feel like map access.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset into the source where the error was detected — not a
    /// line/column (the source is expected to be short enough that a
    /// caller can find the offset by eye, and computing line/column
    /// would need a second pass over bytes already consumed).
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parses a complete DSL document into its single root [`Element`].
/// Leading/trailing whitespace around the root is ignored; anything
/// else outside the root tag (a second root element, stray text) is a
/// [`ParseError`].
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

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            offset: self.pos,
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
            Err(self.error(format!("expected '{}'", expected as char)))
        }
    }

    /// A tag/attribute name: `[A-Za-z_][A-Za-z0-9_-]*` — permissive
    /// enough for both `PascalCase` tags and `kebab-case` attributes
    /// without needing a separate rule for each.
    fn parse_ident(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        match self.peek() {
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => self.pos += 1,
            _ => return Err(self.error("expected an identifier")),
        }
        while matches!(self.peek(), Some(b) if b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            self.pos += 1;
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }

    /// A double-quoted attribute value. `\"` and `\\` are the only
    /// escapes — enough to write a literal quote in an attribute without
    /// needing a full escape grammar.
    fn parse_quoted_string(&mut self) -> Result<String, ParseError> {
        self.expect_byte(b'"')?;
        let mut value = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated string literal")),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(value);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => {
                            value.push('"');
                            self.pos += 1;
                        }
                        Some(b'\\') => {
                            value.push('\\');
                            self.pos += 1;
                        }
                        _ => return Err(self.error("invalid escape in string literal")),
                    }
                }
                Some(_) => {
                    // Safe: we only ever advance `pos` by whole UTF-8
                    // char boundaries below.
                    let rest = std::str::from_utf8(&self.bytes[self.pos..])
                        .map_err(|_| self.error("invalid UTF-8"))?;
                    let ch = rest.chars().next().expect("checked non-empty above");
                    value.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn parse_attrs(&mut self) -> Result<Vec<(String, String)>, ParseError> {
        let mut attrs = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b) if b.is_ascii_alphabetic() || b == b'_' => {
                    let name = self.parse_ident()?;
                    if attrs.iter().any(|(k, _): &(String, String)| k == &name) {
                        return Err(self.error(format!("duplicate attribute '{name}'")));
                    }
                    self.skip_whitespace();
                    self.expect_byte(b'=')?;
                    self.skip_whitespace();
                    let value = self.parse_quoted_string()?;
                    attrs.push((name, value));
                }
                _ => return Ok(attrs),
            }
        }
    }

    fn parse_element(&mut self) -> Result<Element, ParseError> {
        self.expect_byte(b'<')?;
        let tag = self.parse_ident()?;
        let attrs = self.parse_attrs()?;
        self.skip_whitespace();

        if self.peek() == Some(b'/') {
            self.pos += 1;
            self.expect_byte(b'>')?;
            return Ok(Element {
                tag,
                attrs,
                children: Vec::new(),
            });
        }
        self.expect_byte(b'>')?;

        let mut children = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b'<') && self.bytes.get(self.pos + 1) == Some(&b'/') {
                self.pos += 2;
                let closing_tag = self.parse_ident()?;
                if closing_tag != tag {
                    return Err(self.error(format!(
                        "mismatched closing tag: expected '</{tag}>', found '</{closing_tag}>'"
                    )));
                }
                self.skip_whitespace();
                self.expect_byte(b'>')?;
                return Ok(Element {
                    tag,
                    attrs,
                    children,
                });
            }
            if self.peek().is_none() {
                return Err(self.error(format!("unterminated element '<{tag}>'")));
            }
            children.push(self.parse_element()?);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_closing_element_with_attrs() {
        let root = parse(r#"<Mesh path="cube.obj" scale="2" />"#).unwrap();
        assert_eq!(root.tag, "Mesh");
        assert_eq!(root.attr("path"), Some("cube.obj"));
        assert_eq!(root.attr("scale"), Some("2"));
        assert!(root.children.is_empty());
    }

    #[test]
    fn nested_children_in_source_order() {
        let src = r#"
            <Entity name="cube">
                <Mesh path="cube.obj" />
                <RigidBody mass="1.0" />
            </Entity>
        "#;
        let root = parse(src).unwrap();
        assert_eq!(root.tag, "Entity");
        assert_eq!(root.attr("name"), Some("cube"));
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].tag, "Mesh");
        assert_eq!(root.children[1].tag, "RigidBody");
    }

    #[test]
    fn escaped_quote_in_attribute_value() {
        let root = parse(r#"<Label text="say \"hi\"" />"#).unwrap();
        assert_eq!(root.attr("text"), Some("say \"hi\""));
    }

    #[test]
    fn mismatched_closing_tag_is_an_error() {
        let err = parse("<A><B></C></A>").unwrap_err();
        assert!(err.message.contains("mismatched closing tag"));
    }

    #[test]
    fn duplicate_attribute_is_an_error() {
        let err = parse(r#"<Mesh path="a.obj" path="b.obj" />"#).unwrap_err();
        assert!(err.message.contains("duplicate attribute"));
    }

    #[test]
    fn unrecognized_tag_names_parse_fine_here() {
        // Proves the parser is domain-blind: an entirely made-up tag
        // parses identically to a built-in-looking one, since typed
        // meaning is added one layer up, not by this crate.
        let root = parse(r#"<MyGameSpecificWidget hp="100" />"#).unwrap();
        assert_eq!(root.tag, "MyGameSpecificWidget");
        assert_eq!(root.attr("hp"), Some("100"));
    }

    #[test]
    fn trailing_garbage_after_root_is_an_error() {
        let err = parse(r#"<A /> <B />"#).unwrap_err();
        assert!(err.message.contains("end of document"));
    }
}
