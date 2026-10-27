//! Readers for the sfnt tables, one module per table.

pub(crate) mod advances;
pub(crate) mod character_map;
pub(crate) mod directory;
pub(crate) mod glyph_data;
pub(crate) mod header;
pub(crate) mod kerning;
pub(crate) mod loca;
