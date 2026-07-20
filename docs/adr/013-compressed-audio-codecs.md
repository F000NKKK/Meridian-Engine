# ADR 013: Compressed audio via symphonia + libopus, detected by signature

## Status

Accepted — supersedes the "compressed WAV/OGG need an external decoder
crate, added when a concrete asset needs one" deferral in `asset-core`'s
module doc and docs/roadmap.md. The concrete asset exists now
(`examples/assets/audio/`, background music the workspace's examples
actually play).

## Context

`asset-core`'s decoders were hand-rolled for formats simple enough to
parse safely by hand (uncompressed BMP, PCM WAV, minimal OBJ). Music
assets are not that: MP3, Vorbis, FLAC and Opus are each a real DSP
codec — psychoacoustic models, MDCT/CELT/SILK transforms, entropy
coding — a multi-month undertaking *per codec* to hand-roll, squarely
the `wgpu`/`winit`/`cpal` class of decision (ADRs 010/012), not the
"small enough to own" class.

## Decision

**`symphonia` (pure Rust) for MP3, OGG/Vorbis and FLAC.** The ecosystem
standard for pure-Rust audio decoding; enabled features are exactly
`mp3`, `ogg`, `vorbis`, `flac` — no default codecs, nothing speculative.

**Opus through `symphonia-adapter-libopus`.** No mature pure-Rust Opus
decoder exists (symphonia itself demuxes OGG/Opus but cannot decode it),
so Opus uses the reference `libopus` C library — but plugged into
symphonia's own codec registry via the adapter crate, so every
compressed format rides one probe/demux/decode pipeline instead of Opus
getting a hand-written OGG parse next to it. This is the workspace's
first C-library audio dependency; it's built/bundled by `opusic-sys` and
confined entirely inside `asset-core`'s decode path.

**Formats are identified by leading magic bytes, never by file
extension.** `AudioFormat::detect` sniffs RIFF/WAVE, `fLaC`, `ID3`/MPEG
frame sync, and OGG first-packet signatures (`OpusHead` vs `\x01vorbis`);
`AnyAudioDecoder` is the sniffing front door that dispatches to the
per-format decoder (hand-rolled `WavDecoder` included). A file's name is
not part of any decoder's input.

**The bytes-in/CPU-data-out contract is unchanged.** Every decoder still
maps complete in-memory bytes to a complete `AudioData` (interleaved
`i16` + rate + channel count) — no streaming, no I/O, no caching, no
manager types (rule 4), no lifetime policy (ADR 006).

## Alternatives considered

- **Hand-rolling the codecs** — rejected; see "Context".
- **`minimp3`/`lewton`/`claxon` (one crate per codec)** — rejected:
  three separate decode pipelines plus a fourth for Opus, versus
  symphonia's one registry covering all of them with per-codec feature
  flags.
- **Opus via a standalone `ogg` + `libopus` binding pair** — tried
  first, rejected: it duplicates container demuxing symphonia already
  does, and the standalone `opus`/`audiopus_sys` binding fails to build
  with current CMake (its bundled libopus predates CMake 3.5's policy
  floor). `symphonia-adapter-libopus`/`opusic-sys` bundles a current
  libopus and slots into the existing pipeline.
- **Transcoding all assets to WAV at import time** — rejected: music
  assets are exactly the case where 10:1 compression matters on disk,
  and "we support what users actually ship" was the point of the request.

## Consequences

- `asset-core` gains external decoder dependencies (`symphonia`,
  `symphonia-adapter-libopus`) — the deliberate, documented exception
  pattern of ADRs 010/012 applied to codecs.
- Decoding is eager and in-memory (`AudioData` holds the whole decoded
  track). Streaming decode for long music is a real future concern and
  would be a *new* API shape decided on its own merits — not a reason to
  make these decoders half-streaming now.
- PNG/JPEG/glTF stay on the same when-a-concrete-asset-needs-it trigger,
  now with this ADR as the template for accepting them.
