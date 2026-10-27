//! The recursive-descent parser behind `parse`.

use crate::Element;
use crate::ParseError;

pub(crate) struct Parser<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            offset: self.pos,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(crate) fn skip_whitespace(&mut self) {
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

    pub(crate) fn parse_element(&mut self) -> Result<Element, ParseError> {
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
