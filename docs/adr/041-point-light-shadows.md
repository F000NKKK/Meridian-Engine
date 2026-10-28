# ADR 041: shadows from one point light

## Status

Accepted. Extends [ADR 037](037-cascaded-shadow-maps.md) and
[ADR 040](040-per-cascade-casters-and-blending.md).

## Context

Only the first directional light cast a shadow. A point light lit the scene
but shone through everything.

## Decisions

**D1 — one shadowing point light per view.** `PointShadow::for_scene` takes the
scene's first `Light::Point`, as `ShadowLight` takes the first directional
light, and gives it a cube of six shadow maps. `ShadowLight::point` is `None`
when there is no such light or its range does not exceed the near plane. Further point
lights shine unshadowed; a light-index list is future work.

**D2 — six perspective views of 90 degrees.** The faces look along `+x`,
`-x`, `+y`, `-y`, `+z`, `-z` from the light, with a near plane of
`POINT_SHADOW_NEAR` (0.05) and the light's range as far plane. The existing
depth-only shader draws them unchanged: it only applies a matrix.

**D3 — the cube lives in the cascade atlas.** The atlas grows from 2 x 2 to
2 x 5 tiles: tiles 0 to 3 are the cascades, 4 to 9 the faces. Tile `t` is at
column `t % 2`, row `t / 2`, so the shadow pass and the shader share one rule
and the lit shaders need no new binding. At the default 1024 texels a tile,
the atlas is 2048 x 5120 `Depth32Float`, about 42 MB; a larger
`with_shadow_map_size` than 1638 needs a device whose texture limit allows
`5 * size`.

**D4 — casters are culled per face.** The faces' frusta join the cascades' in
the culling of ADR 040, so a face draws only what its pyramid reaches.

**D5 — the shader picks the face by the dominant axis.** For a surface the face
is the one its light-to-surface vector is most aligned with, so no face
selection matrix is needed. The bias works in world units: the surface is
pushed along its normal by 1.5 to 3.5 texels (more at a grazing angle), and the
compare depth is lowered by two texels converted to the perspective depth
the face stores (`near * far / (far - near) * texels / depth^2`). It compares
with `textureSampleCompareLevel`, as the cascades do. Beyond the light's range the
surface is unshadowed.

**D6 — the uniform grows by 432 bytes.** After the environment come six
matrices, the light's position and range, the near plane and the light's index
(`u32::MAX` for none): the lit uniform is 1232 bytes.

## Consequences

- The shadow pass draws up to ten tiles; a scene with no point light draws
  four as before, though the atlas is allocated at its full size.
- The light's `intensity` and `range` shape its glow as before; a point light
  with a very small range gets no shadow.
- Hard-edged like the cascades: one hardware-filtered compare, no PCF kernel or
  soft penumbra.
- Casting from several point lights, and a cube map texture array instead of an
  atlas, are future work.
