# ADR 036: Radiance `.hdr` environment loader

## Status

Accepted. Closes D5 of [ADR 035](035-image-based-lighting.md).

## Context

Image-based lighting ran only on the procedural `Sky`. Lighting a scene with a
photographed or rendered panorama needs an HDR image format. Radiance RGBE
(`.hdr`) is the common one for environment maps: four bytes per pixel (three
mantissas, one shared exponent), optionally run-length encoded per scanline.

## Decisions

**D1 — the decoder is a pure parser in `asset-domain`.** RGBE has no
compression library behind it, so it follows BMP, WAV and OBJ rather than the
codec adapters (`asset-png`, `asset-jpeg`). `HdrDecoder` implements
`Decoder<HdrImageAsset>`; `HdrImageAsset` holds linear `[f32; 3]` pixels
(`mantissa * 2^(exponent - 136)`, exponent 0 is black). No new crate, so the
dependency rules do not change.

**D2 — HDR is not an `ImageFormat`.** `ImageFormat` and `AnyImageDecoder` map
magic bytes to the 8-bit `ImageAsset` that textures use. An `.hdr` file
carrying radiance above 1.0 must not be quantised to RGBA8, so it has its own
asset type and its own `HdrDecoder::detect` (`#?RADIANCE` or `#?RGBE`).

**D3 — scope is what environment maps use.** Accepted: `FORMAT=32-bit_rle_rgbe`
(or no format line), resolution line `-Y height +X width`, flat scanlines and
new-style RLE scanlines (width 8 to 32767). Refused with a typed error:
other orientations, other formats, more than 2^26 pixels. Runs that overrun a
scanline, zero-length runs and truncated data are `DecodeError`s; pixels are
appended scanline by scanline, so a lying header cannot force an allocation
larger than the data actually decoded.

**D4 — loading is `GraphicsSession::load_environment(path)`.** It reads the
file, decodes it, builds the map with `EnvironmentMap::from_equirect` and
hands it to `set_environment`. The panorama must use the equirectangular
convention of ADR 035. `gltf_viewer` takes `MERIDIAN_ENV=<file>.hdr`.

## Consequences

- Any equirectangular Radiance panorama lights a scene; `Sky` remains the
  default when no file is given.
- Old-style RLE (`1 1 1 n` repeat pixels) is not recognised: such a file is read
  as flat data and ends in a truncation error or wrong pixels. Rare in practice.
- EXR and a cube-map environment source are still not supported.
