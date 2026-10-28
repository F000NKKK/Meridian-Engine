# ADR 039: the environment as a visible backdrop

## Status

Accepted. Follows [ADR 035](035-image-based-lighting.md) and
[ADR 038](038-cube-map-environment-source.md).

## Context

The environment lit the scene and filled its reflections, but the frame behind
the geometry was always one flat clear colour, so a reflective helmet mirrored a
sky the viewer could not see.

## Decisions

**D1 — the backdrop is a property of the scene.** `Scene3d::background` is a
`Background`: `Clear` (the default, the frame's clear colour) or
`Environment { blur }`, the recorder's environment seen through the camera.
`blur` runs from 0 (the sharpest prefiltered level) to 1 (the roughest) and
picks the mip level. It is on the scene, not on the `FrameRecorder` port, so
the port does not change and a second view can choose differently. Without an
environment set, `Environment` shows the clear colour.

**D2 — one fullscreen triangle first in the main pass.** `SkyPass` draws before
the scene's geometry into the HDR target. Its pipeline declares the pass's depth
attachment but neither tests nor writes it, so everything drawn later lands
over the sky whatever its depth. The shader rebuilds each pixel's view ray from
the camera's world axes and the half-extents of its view plane (read from the
perspective projection) and looks it up in the same equirectangular texture the
lit shaders sample.

**D3 — no tone mapping or bloom special cases.** The sky is HDR radiance in the
scene target, so exposure and the tone curve treat it like any lit surface. A
bright sun in it does not bloom: bloom is fed by emissive geometry only.

**D4 — perspective cameras only.** A camera with an orthographic or identity
projection has no view rays, so it keeps the clear colour.

## Consequences

- `gltf_viewer` shows its procedural sky or the loaded panorama or cube map
  behind the model (`MERIDIAN_ENV=none` returns the flat colour).
- The sky is drawn at full screen size every frame it is on: one texture
  lookup per pixel.
- It is sampled bilinearly from the environment's own levels: the sharpest is
  at most 256 texels wide, so a close-up of a high-resolution panorama looks
  soft. A separate full-resolution backdrop texture is future work.
- A backdrop that moves with the camera position (a skybox at infinity) is
  what this is; a finite or parallax-corrected one is not supported.
