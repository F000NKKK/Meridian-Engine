//! [`SaveHeader`]: what a save says of itself.

use super::SaveError;
use super::envelope_reader::read_header_and_scene;
use crate::SAVE_FORMAT;

/// What a save says of itself: the envelope's `format`, the `game` that
/// wrote it, and when (`created`). Reading it takes the first bytes of the
/// file, not the scene or the game's state, so a save list can show a
/// hundred saves cheaply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveHeader {
    /// The envelope format ([`SAVE_FORMAT`] when written by this crate).
    pub format: u32,
    /// The game that wrote it (its own identifier, free text).
    pub game: String,
    /// When, as seconds since the Unix epoch. The caller says: the data
    /// layer has no clock.
    pub created: i64,
}

impl SaveHeader {
    /// The header of a save by `game` made at `created` (Unix seconds).
    #[must_use]
    pub fn new(game: impl Into<String>, created: i64) -> Self {
        SaveHeader {
            format: SAVE_FORMAT,
            game: game.into(),
            created,
        }
    }

    /// Reads the header of a save's `bytes` without decoding its scene or
    /// state: the bytes after the header fields are not looked at, so a
    /// damaged scene or state is found by loading, not by this.
    ///
    /// # Errors
    ///
    /// [`SaveError`] for bytes that are not a save, a field that is missing
    /// or of the wrong type, or a `format` this crate doesn't read.
    pub fn read(bytes: &[u8]) -> Result<Self, SaveError> {
        read_header_and_scene(bytes, false).map(|(header, _)| header)
    }
}
