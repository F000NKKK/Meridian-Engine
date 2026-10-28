# ADR 045: spot lights and their shadows

## Status

Accepted. Builds on [ADR 041](041-point-light-shadows.md) and
[ADR 043](043-nearest-point-light-shadows.md), whose slots it shares.

## Context

The engine had a directional and a point light, and no light that shines into
a cone. A flashlight, a stage lamp or a window of light needs one, and its
shadow needs a map that covers the cone instead of a cube around the light.

## Decisions

**D1 — `Light::Spot`.** A point light with a `direction` (a motor whose X axis
is the travel direction, as for `Directional`), an `inner_angle` and an
`outer_angle`, both half-angles in radians. It falls off with distance like a
point light and across the cone by `saturate((cos - cos_outer) / (cos_inner -
cos_outer))` squared. `spot_cone` limits the outer angle to
`SPOT_MAX_OUTER_ANGLE` (1.4 rad, about 80 degrees, so the shadow view stays a
finite perspective projection) and the inner one to the outer. A wider cone is
shaded as the limit.

**D2 — a spot light's shadow is one perspective view.** `SpotShadow` holds the
view-projection of a camera at the light, looking along its direction, with a
field of view of twice the outer angle, `SPOT_SHADOW_NEAR` and the light's
range as planes. A single frustum culls its casters, as for a cascade.

**D3 — spot and point lights share the shadow slots.** `LocalShadow` is a
`Point` cube or a `Spot` view, and `LocalShadow::nearest` ranks every point and
spot light by distance from the camera and keeps `MAX_LOCAL_SHADOWS` (4, the
former `MAX_POINT_SHADOWS`) in slots, as ADR 043 did for points alone. The
lights beyond the budget shine unshadowed. `ShadowLight::points` became
`locals`.

**D4 — a spot light's map is a full tile over its slot's block.** A slot owns
a 2 x 3 block of half-size faces (ADR 043); a spot light uses the origin of
that block as one square tile as large as a cascade tile, the same resolution
for a cone as a cascade gets. The faces under it are not drawn while the slot
holds a spot light. So the atlas does not grow, `tile_rect` has one more
tile per slot (`spot_tile`) and the shader repeats that arithmetic.

**D5 — the uniform grows by the spot's data, not by a new array.** Each
`LightData` gains a `spot` vec4 (direction, cosine of the outer angle) and the
cone's scale in `range_pad.y`: 64 bytes per light instead of 48, 64 more in
all. A spot light's matrix is the first of its slot's six. In `shade` the slot
loop picks `spot_shadow_factor` when the light's kind is 2 and
`point_shadow_factor` otherwise.

**D6 — `spot_shadow_factor` biases like the point lookup.** The surface is
pushed along its normal by a couple of texels, which grow with the slope, and
compared with a depth bias of the same size converted to perspective depth. A
texel's world size comes from the depth along the axis and the cone's
tangent.

## Consequences

- Scenes may use spot lights; every `match` on `Light` handles a third case.
- Up to four local lights, points and spots together, cast shadows. A scene
  with four points and a spot gives the spot a shadow only if it is among the
  nearest four.
- A spot light's shadow has the resolution of a cascade tile across its whole
  cone, so a wide cone has coarse texels; the budget stays four.
- A light changing rank swaps slots and its shadow may pop, as in ADR 043.
- Spot lights are shaded but cast no cone-shaped volumetric light; there is no
  cookie texture.
