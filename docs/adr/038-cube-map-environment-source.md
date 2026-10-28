# ADR 038: cube map as an environment source

## Status

Accepted. Follows [ADR 035](035-image-based-lighting.md) (image-based
lighting) and [ADR 036](036-radiance-hdr-environment-loader.md) (`.hdr`
loader).

## Context

Panoramas are one way to ship an environment. The other is six square faces,
the form skybox sets and many renderers export. ADR 035 left a cube-map
environment open.

## Decisions

**D1 — a cube map is a source, not a stored form.** `CubeFaces` holds six
square faces of linear radiance; `EnvironmentMap::from_cube` resamples them to
an equirectangular image (four times a face wide, at most 1024) and runs the
existing prefilter. The stored environment and its GPU texture stay
equirectangular, so no shader, bind group or uniform changes. A GPU cube
texture would not make the lookups cheaper, since the prefiltering is
done once on the CPU, and would add a second sampling path to every lit shader.

**D2 — face order and orientation.** Faces are `+x, -x, +y, -y, +z, -z`, each
stored row-major from its top row, in the OpenGL and Vulkan cube map
orientation that standard skybox sets use. `CubeFaces::sample` implements the
specification's direction-to-face table; the world is the engine's (`y` up, the
equirectangular centre looking along `-z`).

**D3 — sampling is bilinear inside a face only.** A lookup near a face edge
clamps to that face and does not blend with its neighbour. The resampled
image is at most 1024 wide and then averaged down to 256 for the sharpest
level, so seams are far below a texel there; a seamless filter is not worth its
code.

**D4 — loading is `GraphicsSession::load_environment_cube(paths)`.** Six files
in the face order. A face is a Radiance `.hdr` (detected by its signature) or
any image the engine decodes, read as sRGB and converted to linear. All six
must be square and the same size. `gltf_viewer` takes six comma-separated
paths in `MERIDIAN_ENV`.

## Consequences

- Skybox sets work as lighting; an LDR set lights as a dim sky since it has no
  radiance above 1.0, so HDR faces are the useful input.
- Cross-layout (one image with six faces) and DDS/KTX cube containers are not
  read; split them into faces.
- Rendering the cube as a visible skybox is not part of this change; the
  background is still the scene's clear colour.
