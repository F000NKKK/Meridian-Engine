# ADR 023: Our own font stack (`meridian-font-core`)

## Status

Proposed (2026-10-27). Implemented so far: the TrueType parser and
outlines (§2, §3); rasterizer, atlas and wiring (§4, §5, §7) follow.

## Context

The owner decided that text is drawn by **our own rasterizer**: no
`fontdue`, `ab_glyph` or `rusttype`. `meridian-ui-core` already asks for
text size through the `TextMeasurer` port ([ADR 018](018-ui-architecture.md)),
and `meridian-ui-render` ([ADR 021](021-ui-extraction-and-extensibility.md))
emits sprites from a texture, so the missing piece is: font bytes, to glyph
outlines, to coverage bitmaps, to an atlas texture, with the metrics layout
needs. The Yog-Mod-Loader `yog-font` shows the shape (outlines as drawings,
shaping separate from rasterization), but is tied to Yog's graphics types.

## Decision

**1. One new domain crate, `meridian-font-core`.** Pure CPU, no I/O, no
`async`, no GPU. It depends on no workspace crate (`gac-core` is not needed: glyph
geometry is 2D font units, not spatial algebra). It belongs to the `ui`
context. It does not read
files: callers pass bytes (`asset-core` or the game owns loading and
lifetime, ADR 004/011).

**2. Parsing, bounds-checked.** `FontFace::parse(&[u8])` reads TrueType
(`glyf` outlines) first: `head`, `maxp`, `hhea`, `hmtx`, `cmap` (formats 4
and 12), `loca`, `glyf` (simple and composite glyphs), `kern`. OpenType/CFF
outlines are a later step behind the same `Outline` type. Every offset and
length is checked; malformed bytes are a typed `FontError`, never a panic
or an unbounded allocation. A synthetic font built in the tests is the
fixture; no third-party font is committed.

**3. Outlines.** `Outline` is a list of contours of line and quadratic
segments in font units (cubics from CFF are flattened or converted at
parse time). It is a public type, so a game can supply outlines from any
source (an icon font, a vector asset) without a font file.

**4. Rasterizer.** `rasterize(outline, scale, subpixel_offset)` returns an
8-bit coverage bitmap using exact-area scanline accumulation (non-zero
winding), with its bearing and size. Anti-aliasing is area coverage; no
hinting in the first version. SDF is a later option behind the same cache.

**5. Atlas and cache.** `GlyphAtlas` packs coverage bitmaps into a fixed
size page with a shelf packer and reports each glyph's pixel rect, from
which the renderer derives the normalized `source_rect` rendering expects.
`GlyphCache` maps `(face, glyph, size bucket)` to an atlas slot with
eviction by page reset. It produces bytes; uploading them to a texture is
the renderer's job (`TextureRegistry`, team/rendering), and lifetime goes
through `resource-core` handles.

**6. Shaping scope.** Left-to-right, one cluster per `char`, advance from
`hmtx`, pair adjustment from `kern`, line breaks on `\n`. Ligatures,
bidirectional text and complex scripts are out of scope until a concrete
need; the `Shaper` is a trait so a game or a later crate can replace it.

**7. Wiring.** `ui-render` depends on `font-core`, implements
`TextMeasurer` over a `FontFace` (so layout and drawing share the same
metrics), and extracts a `TextBlock` into one sprite per glyph using
`Foreground` as the tint. `ui-core` stays free of fonts.

**8. Extensibility (ADR 021 §6).** `Outline`, `Shaper` and the atlas are
public and independent: a game can rasterize its own outlines, bring its own
shaper, or pack its own bitmaps into the atlas without using the rest.

## Consequences

- Text is testable without a GPU: parse, rasterize and pack are pure
  functions with byte-exact expectations.
- We own the parser and rasterizer, which is a real maintenance cost
  (hinting, CFF and complex scripts are not free); the first version
  deliberately stops at Latin-style text.
