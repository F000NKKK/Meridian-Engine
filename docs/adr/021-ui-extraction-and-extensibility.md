# ADR 021: UI output to `Scene2D` and the extensibility rule (`meridian-ui-render`)

## Status

Proposed (2026-10-27). Extraction of background fills is implemented in
`meridian-ui-render`; text, borders, images and custom paint are not.

## Context

[ADR 018](018-ui-architecture.md) built the retained tree and layout in
`meridian-ui-core` and deliberately left output open. The owner asked for
output first, so that every later control can be looked at and tested, and
for the output to live outside `ui-core`. The owner also set a product
rule for the whole UI stack: **we give developers a tool, not shackles**.
Unity and Unreal let a team replace the layout, draw custom visuals,
intercept input and bypass the framework entirely; the UI must not push
anyone into a corner where the only way out is forking the engine.

## Decision

**1. A separate adapter crate.** `meridian-ui-render` (layer `infra`,
context `ui`) depends on `ui-core`, `graphics-core` and `gac-core`. It
reads a laid-out `UiTree` and produces a `graphics-core::Scene2d`. It
calls no driver and owns no GPU resource; submission is the existing
graphics path. `ui-core` stays free of graphics.

**2. Extraction contract.** `extract_scene(tree, root, surface, paint)`
walks the tree in document order. A later node paints over an earlier
one, and children paint over their parent, so the running sprite count is
the `layer`. Subtrees that are not `Visible` are skipped, including
`Hidden` ones (they keep their layout slot but draw nothing). The camera is
`Camera2d::from_surface_size`: pixels, origin top-left, `y` down, the same
convention as layout.

**3. Colour.** `ui_core::Color` is linear-in-name only: four `f32`
channels as written in markup (`#RGB`, `#RRGGBB`, `#RRGGBBAA`,
`Transparent`, `Black`, `White`), passed to the sprite `tint` unchanged.
Colour management is the renderer's concern. `Background` (any element)
and `Foreground` (`TextBlock`) are ordinary properties.

**4. Sprite convention.** A solid rectangle is a `Sprite` sampling a 1x1
white texture (`UiPaint::fill`, supplied by the caller) tinted with the
colour. `Sprite::frame` is the **centre** of the quad in surface pixels
and `size` its extent. Nothing in `graphics-core` consumes `Scene2d` yet:
the sprite renderer, texture upload and alpha blending are requests to
team/rendering, and this convention is what they build against.

**5. Text.** Text is drawn from our own rasterizer: TTF/OTF outline
parsing, rasterization and a glyph atlas behind the existing
`TextMeasurer` port, in its own crate(s) and ADR. Until then `TextBlock`
lays out through the port and extraction emits nothing for it.

**6. The extensibility rule.** The UI is a toolbox. The following are
design requirements for `ui-core`, `ui-render` and every control, and a
change that breaks one needs an ADR:

- Markup is optional. Everything `.mel` can do is reachable from plain
  Rust through `UiTree` (create, set, bind); markup is generated code on
  top of the same API.
- Custom elements, panels and controls are defined through public traits
  (measure, arrange, paint, input), not by editing a closed enum. Closed
  enums that user code must extend (`ElementKind`, `PropertyValue`) get an
  extension variant or are replaced by a trait before controls ship.
- A custom draw hook (`OnPaint`-style, appending sprites or raw
  `Scene2d` content) exists beside the stock extraction, and
  `extract_scene` is a building block, not the only way to produce a
  `Scene2d`.
- Input can be intercepted before it reaches the target (tunnelling, see
  ADR 018 follow-up) and handled without the built-in focus rules.
- Styles and templates replace default visuals; no control hard-codes its
  look.
- Unused parts cost nothing: a game can use only layout, only hit
  testing, or only extraction.

## Consequences

- The UI is testable without a GPU: extraction is a pure function whose
  output is asserted in unit tests.
- `Scene2d` is the seam. Rendering can land independently, and a CPU
  compositor for headless screenshots is possible behind the same seam.
- Extensibility is a standing cost: the closed enums in ADR 018 will
  change, and that is accepted now, before controls multiply the
  call sites.
