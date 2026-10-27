//! Literals that are one of a few names.

use super::LiteralError;
use crate::layout::value::{Alignment, Dock, Orientation, Visibility};

/// The value whose name `text` spells.
fn pick<T: Copy>(
    text: &str,
    expected: &'static str,
    names: &[(&str, T)],
) -> Result<T, LiteralError> {
    let trimmed = text.trim();
    names
        .iter()
        .find(|(name, _)| *name == trimmed)
        .map(|(_, value)| *value)
        .ok_or_else(|| LiteralError::invalid(expected, trimmed))
}

/// `Left`/`Top` mean `Start` and `Right`/`Bottom` mean `End`, so the WPF
/// spellings of either axis work.
pub(in crate::property) fn alignment(text: &str) -> Result<Alignment, LiteralError> {
    let names = [
        ("Start", Alignment::Start),
        ("Left", Alignment::Start),
        ("Top", Alignment::Start),
        ("Center", Alignment::Center),
        ("End", Alignment::End),
        ("Right", Alignment::End),
        ("Bottom", Alignment::End),
        ("Stretch", Alignment::Stretch),
    ];
    pick(text, "Start, Center, End or Stretch", &names)
}

pub(in crate::property) fn orientation(text: &str) -> Result<Orientation, LiteralError> {
    let names = [
        ("Vertical", Orientation::Vertical),
        ("Horizontal", Orientation::Horizontal),
    ];
    pick(text, "Vertical or Horizontal", &names)
}

pub(in crate::property) fn dock(text: &str) -> Result<Dock, LiteralError> {
    let names = [
        ("Left", Dock::Left),
        ("Top", Dock::Top),
        ("Right", Dock::Right),
        ("Bottom", Dock::Bottom),
    ];
    pick(text, "Left, Top, Right or Bottom", &names)
}

pub(in crate::property) fn visibility(text: &str) -> Result<Visibility, LiteralError> {
    let names = [
        ("Visible", Visibility::Visible),
        ("Hidden", Visibility::Hidden),
        ("Collapsed", Visibility::Collapsed),
    ];
    pick(text, "Visible, Hidden or Collapsed", &names)
}
