//! [`BracketDepth`]: bracket nesting and `'...'` quoting while scanning a
//! markup-extension value.

use super::markup_value::{MarkupValueError, MarkupValueErrorKind};

#[derive(Default)]
pub(crate) struct BracketDepth {
    open: Vec<char>,
    quoted: bool,
}

impl BracketDepth {
    /// The byte index of the `}` that closes the `{` at index 0.
    pub(crate) fn closing_brace(value: &str) -> Result<usize, MarkupValueError> {
        let mut depth = Self::default();
        for (index, ch) in value.char_indices() {
            if depth
                .step(ch)
                .map_err(|kind| MarkupValueError::at(kind, index))?
                && ch == '}'
            {
                return Ok(index);
            }
        }
        Err(MarkupValueError::at(
            MarkupValueErrorKind::Unbalanced,
            value.len(),
        ))
    }

    /// Splits on commas that are not inside brackets, braces or quotes.
    pub(crate) fn split_top_level(text: &str) -> Result<Vec<&str>, MarkupValueError> {
        let mut parts = Vec::new();
        let mut depth = Self::default();
        let mut start = 0;
        for (index, ch) in text.char_indices() {
            if ch == ',' && depth.is_top() {
                parts.push(&text[start..index]);
                start = index + 1;
                continue;
            }
            depth
                .step(ch)
                .map_err(|kind| MarkupValueError::at(kind, index))?;
        }
        if !depth.is_top() {
            return Err(MarkupValueError::at(
                MarkupValueErrorKind::Unbalanced,
                text.len(),
            ));
        }
        parts.push(&text[start..]);
        Ok(parts)
    }

    fn is_top(&self) -> bool {
        self.open.is_empty() && !self.quoted
    }

    /// Feeds one char; `true` when it closed the outermost bracket.
    fn step(&mut self, ch: char) -> Result<bool, MarkupValueErrorKind> {
        if ch == '\'' {
            self.quoted = !self.quoted;
            return Ok(false);
        }
        if self.quoted {
            return Ok(false);
        }
        match ch {
            '(' => self.open.push(')'),
            '[' => self.open.push(']'),
            '{' => self.open.push('}'),
            ')' | ']' | '}' => {
                if self.open.pop() != Some(ch) {
                    return Err(MarkupValueErrorKind::Unbalanced);
                }
                return Ok(self.open.is_empty());
            }
            _ => {}
        }
        Ok(false)
    }
}
