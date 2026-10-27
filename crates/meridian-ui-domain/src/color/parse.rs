//! Color literals: `#RGB`, `#RRGGBB`, `#RRGGBBAA` and a few names.

use super::Color;
use crate::property::LiteralError;

const EXPECTED: &str = "#RGB, #RRGGBB, #RRGGBBAA, Transparent, Black or White";

impl Color {
    /// Parses a color literal.
    ///
    /// # Errors
    ///
    /// [`LiteralError::Invalid`] when `text` is not one of the forms above.
    pub fn parse(text: &str) -> Result<Self, LiteralError> {
        let trimmed = text.trim();
        match trimmed {
            "Transparent" => return Ok(Self::TRANSPARENT),
            "Black" => return Ok(Self::BLACK),
            "White" => return Ok(Self::WHITE),
            _ => {}
        }
        let hex = trimmed.strip_prefix('#').ok_or_else(|| invalid(trimmed))?;
        let digits: Vec<u8> = hex
            .chars()
            .map(|digit| {
                digit
                    .to_digit(16)
                    .and_then(|value| u8::try_from(value).ok())
            })
            .collect::<Option<_>>()
            .ok_or_else(|| invalid(trimmed))?;
        match digits[..] {
            [r, g, b] => Ok(Self::from_rgba8(r * 17, g * 17, b * 17, 255)),
            [r1, r2, g1, g2, b1, b2] => Ok(Self::from_rgba8(
                r1 * 16 + r2,
                g1 * 16 + g2,
                b1 * 16 + b2,
                255,
            )),
            [r1, r2, g1, g2, b1, b2, a1, a2] => Ok(Self::from_rgba8(
                r1 * 16 + r2,
                g1 * 16 + g2,
                b1 * 16 + b2,
                a1 * 16 + a2,
            )),
            _ => Err(invalid(trimmed)),
        }
    }
}

fn invalid(found: &str) -> LiteralError {
    LiteralError::Invalid {
        expected: EXPECTED,
        found: found.to_string(),
    }
}
