# meridian-font-core

Our own font stack, CPU only ([ADR 023](../../docs/adr/023-own-font-stack.md)).
Today it reads a TrueType face from bytes: metrics, character map (cmap
formats 4 and 12), kerning, and glyph outlines (simple and composite) as
quadratic contours in font units. Malformed input is a `FontError`, never a
panic. A rasterizer, glyph atlas and shaper follow.
