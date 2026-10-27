//! A parsed font face.

use crate::cmap::Cmap;
use crate::directory::Directory;
use crate::glyf::Glyf;
use crate::head::Header;
use crate::kerning::Kerning;
use crate::{FontError, GlyphId, Outline, advances, loca};

/// A TrueType face: metrics, character map and glyph outlines, all in
/// font units. Parsing copies what it needs, so the face owns its data.
#[derive(Debug, Clone)]
pub struct FontFace {
    header: Header,
    advances: Vec<u16>,
    cmap: Cmap,
    kerning: Kerning,
    glyf: Glyf,
}

impl FontFace {
    /// Reads a face from the bytes of a `.ttf` file.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the data is not a TrueType font this version can
    /// read, or contradicts itself.
    pub fn parse(bytes: &[u8]) -> Result<Self, FontError> {
        let directory = Directory::parse(bytes)?;
        let header = Header::parse(
            directory.required(*b"head", "head")?,
            directory.required(*b"maxp", "maxp")?,
            directory.required(*b"hhea", "hhea")?,
        )?;
        let advances = advances::parse(
            directory.required(*b"hmtx", "hmtx")?,
            header.glyph_count,
            header.metric_count,
        )?;
        let cmap = Cmap::parse(directory.required(*b"cmap", "cmap")?)?;
        let glyf_bytes = directory.required(*b"glyf", "glyf")?;
        let offsets = loca::parse(
            directory.required(*b"loca", "loca")?,
            header.glyph_count,
            header.long_loca,
            glyf_bytes.len(),
        )?;
        let kerning = directory
            .table(*b"kern")?
            .map(Kerning::parse)
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            header,
            advances,
            cmap,
            kerning,
            glyf: Glyf::new(glyf_bytes.to_vec(), offsets),
        })
    }

    /// Font units per em square; divide by it to normalise a measurement.
    #[must_use]
    pub const fn units_per_em(&self) -> u16 {
        self.header.units_per_em
    }

    /// Distance from the baseline to the top of the line, in font units.
    #[must_use]
    pub const fn ascender(&self) -> i16 {
        self.header.ascender
    }

    /// Distance from the baseline to the bottom of the line; usually negative.
    #[must_use]
    pub const fn descender(&self) -> i16 {
        self.header.descender
    }

    /// Extra space between lines, in font units.
    #[must_use]
    pub const fn line_gap(&self) -> i16 {
        self.header.line_gap
    }

    /// How many glyphs the face holds.
    #[must_use]
    pub const fn glyph_count(&self) -> u16 {
        self.header.glyph_count
    }

    /// The glyph for `c`, or `None` when the face does not cover it.
    #[must_use]
    pub fn glyph_id(&self, c: char) -> Option<GlyphId> {
        self.cmap.glyph(c)
    }

    /// The horizontal advance of `glyph` in font units, `None` for an id
    /// beyond the face.
    #[must_use]
    pub fn advance(&self, glyph: GlyphId) -> Option<u16> {
        self.advances.get(usize::from(glyph.0)).copied()
    }

    /// The adjustment to the advance of `left` when `right` follows it, in
    /// font units; `0` when the face has no such pair.
    #[must_use]
    pub fn kerning(&self, left: GlyphId, right: GlyphId) -> i16 {
        self.kerning.adjustment(left, right)
    }

    /// The shape of `glyph`; empty for glyphs that draw nothing.
    ///
    /// # Errors
    ///
    /// [`FontError`] for an id beyond the face or malformed glyph data.
    pub fn outline(&self, glyph: GlyphId) -> Result<Outline, FontError> {
        self.glyf.outline(glyph.0)
    }
}
