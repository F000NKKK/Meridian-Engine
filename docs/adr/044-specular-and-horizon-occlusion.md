# ADR 044: specular and horizon occlusion

## Status

Accepted. Refines [ADR 033](033-pbr-texture-maps.md) and
[ADR 035](035-image-based-lighting.md), which dimmed the whole ambient term by
the occlusion map.

## Context

The occlusion map darkens ambient light, diffuse and reflected alike. A smooth
metal in a crevice then loses its reflection as if it were chalk, while a
reflection that points into a surface (which a normal map makes common) still
shows the sky behind it.

## Decisions

**D1 — ambient diffuse and ambient specular are separate terms.** `shade`
keeps the diffuse part multiplied by the occlusion map and multiplies the
reflected part by its own factor. Direct lights are unchanged.

**D2 — the reflected part uses specular occlusion.** `specular_occlusion` is
the Lagarde and de Rousiers approximation,
`saturate(pow(n_dot_v + ao, exp2(-16 * roughness - 1)) - 1 + ao)`: a rough
surface is occluded like its diffuse light, a smooth one only where view and
occluder line up.

**D3 — a reflection into the geometric surface fades out.** `horizon_occlusion`
is `saturate(1 + 1.3 * dot(reflected, geometric_normal))` squared. The
geometric normal is the interpolated vertex normal, before the normal map;
`shade` takes it as a new argument.

## Consequences

- Smooth surfaces keep their reflections in occluded areas; normal-mapped
  surfaces no longer reflect sky through themselves.
- The occlusion map is still one scalar per texel, with no bent normal or
  directional information, so the approximation is only as good as that scalar.
- No change to uniforms, bindings or pipelines; the change is two WGSL
  functions and one extra argument.
