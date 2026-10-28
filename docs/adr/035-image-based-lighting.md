# ADR 035: image-based lighting from a prefiltered environment

## Status

Accepted. Builds on the shading of [ADR 032](032-metallic-roughness-materials.md)
and the HDR scene target of [ADR 034](034-hdr-scene-target-and-tone-mapping.md).

## Context

Lit surfaces got their ambient light from a two-colour hemisphere gradient,
looked up by the normal for diffuse and by the reflection vector for
specular. Smooth metal therefore reflected a flat gradient, never a sky with a
bright sun, and rough surfaces took the same sharp lookup as smooth ones.
Image-based lighting replaces that with an environment: radiance from every
direction, filtered the way a surface of a given roughness integrates it.

## Decisions

**D1 — the environment is data built on the CPU, in rendering-domain.**
`EnvironmentMap` is built from an equirectangular image of linear radiance
(`from_equirect`, values may exceed 1.0) or from a procedural `Sky` (zenith,
horizon and ground colours plus an optional sun disc). It holds six specular
levels, level `i` prefiltered for roughness `i / 5` by GGX importance sampling
(64 samples per texel, read from a box-filtered pyramid of the source so few
samples stay smooth), and nine spherical-harmonic coefficients of diffuse
irradiance. Level `i` is `width >> i` by `height >> i` so the levels are
exactly the mip chain of a texture of the first. The domain has no GPU types,
so the filtering is unit-tested: a uniform sky stays uniform at every level and
in irradiance, and a sun disc spreads as roughness grows.

**D2 — one equirectangular texture with mip levels, not a cube map.** The
adapter uploads the chain as one `Rgba16Float` 2D texture (f32 to half is done
by hand, overbright values clamp to the largest finite half). The shader
reads `textureSampleLevel` at `roughness * max_lod`, wrapping around the
horizon and clamping at the poles. The mapping is `direction_to_uv` in the
domain and is mirrored in WGSL. A cube map avoids the texel pinching at the poles
that an equirectangular image has, and needs a cube texture type through the
GPU crates; that is left for when an environment image needs it.

**D3 — shading.** With an environment set, ambient light is
`irradiance(n) * diffuse_color + prefiltered(reflect(-v, n), roughness) *
(f0 * A + B)`, with `A` and `B` from Karis' analytic fit of the split-sum BRDF
(no lookup texture), all multiplied by occlusion as before. The nine
coefficients are stored as irradiance / pi, which matches this renderer's
convention that diffuse carries no 1/pi. Without an environment the hemisphere
gradient is used exactly as before, selected by a flag in the uniform, so every
existing scene is unchanged.

**D4 — a port method sets it.** `FrameRecorder::set_environment(Option<&EnvironmentMap>)`.
`GpuFrameRecorder` forwards it to `SceneRenderer::set_environment`, which
uploads the texture and rebuilds the flat lit pipelines' cached bind groups.
While none is set a 1x1 black texture stands in so every bind group is valid.
The lit uniform grows from 416 to 576 bytes (`env_sh`: nine `vec4`, then
`env_params`: enabled, highest mip level). Lit pipelines bind the environment
at 3 and 4 (flat colour) or 9 and 10 (textured).

**D5 — loading an environment image is not part of this ADR** (done in
[ADR 036](036-radiance-hdr-environment-loader.md)). The procedural `Sky` was
the only source until a Radiance `.hdr` decoder existed in the asset layer; it only has to produce the `width`, `height`, texel list that
`from_equirect` takes.

## Consequences

- Metals reflect the sky and its sun, rough metals a blurred version, and
  dielectrics get a Fresnel-weighted sky reflection at grazing angles.
  `gltf_viewer` lights DamagedHelmet with a sky matching its sun
  (`MERIDIAN_ENV=none` turns it off).
- Building a map costs about a million lookups (a 256x128 source), done once
  at setup, not per frame.
- Specular occlusion and horizon occlusion are not modelled: occlusion maps dim
  the whole ambient term, as in ADR 033, and a reflection can show sky through
  a nearby wall.
- The sun in a `Sky` and a directional light are separate: a scene that wants
  both must place them consistently.
- One more texture and sampler on every lit pipeline; the textured one now
  binds eleven resources.
