# ADR 043: shadows from the nearest point lights

## Status

Accepted. Refines [ADR 041](041-point-light-shadows.md), which shadowed one
point light.

## Context

ADR 041 gave only the scene's first point light a shadow cube. A scene with
several lamps lit them all but shadowed one, and which one depended on list
order, not on what the viewer sees.

## Decisions

**D1 — the nearest `MAX_POINT_SHADOWS` (4) point lights cast shadows.**
`PointShadow::nearest` takes the scene's point lights that have room for a
cube, orders them by distance from the camera and keeps four, nearest first.
`ShadowLight::points` holds them in slots; an unused slot is `None`. Further
lights shine unshadowed. The choice is made every frame, so a light entering
the nearest four takes a slot and its shadow appears; there is no fade, so a
light changing rank can make a shadow pop.

**D2 — a cube's faces are half the size of a cascade tile.** The atlas keeps
its size (`2 * tile` by `5 * tile`, 42 MB at the default 1024). Below the 2 x 2
cascade tiles each point light owns a block of 2 x 3 faces of `tile / 2` texels
(512 by default), the blocks two across. The four blocks fill the three
remaining tile rows exactly, so four lights cost what one light cost under
ADR 041, at half its resolution per face. `tile_rect` is the one place that
turns a tile index into texels; the shader repeats its arithmetic.

**D3 — the shader finds a light's slot by its index.** The lit uniform holds,
per slot, the six face matrices, the light's position and range, and the
index of the light in the scene's lights (`u32::MAX` for an empty slot). While
shading light `i` the shader asks each of the four slots whether it is `i`
and, if so, multiplies the light's contribution by that slot's visibility. The
uniform grows from 1232 to 2032 bytes.

**D4 — culling and the depth pass treat every face as a tile.** A frame draws
the four cascades and up to 24 faces, each culled against its own volume as in
ADR 040. A scene with no point light draws four tiles.

## Consequences

- Several lamps cast shadows at once, and the ones nearest the viewer win a
  slot.
- Every face is sharper than a cascade by area but only half as wide as one:
  a point light's shadows are softer than ADR 041's.
- Worst case the pass draws 28 tiles of casters per frame; the cost grows
  with the number of casters near each light, not with the budget alone.
- Hysteresis for slot changes, a fade, and a budget by screen coverage instead
  of distance are future work.
