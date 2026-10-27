# meridian-text-domain

Our own font stack, CPU only ([ADR 023](../../docs/adr/023-own-font-stack.md)).
It reads a TrueType face from bytes (metrics, character map formats 4 and 12,
kerning, simple and composite glyph outlines as quadratic contours),
rasterizes outlines with area-exact anti-aliasing, and packs the bitmaps into
a shelf-packed glyph atlas with a per-size cache. Malformed input is a
`FontError`, never a panic. Uploading the atlas to a texture belongs to the
renderer.
