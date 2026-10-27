# Segment: audio (`team/audio`)

Ownership note and working backlog for the audio segment. Status of record
stays in [roadmap.md](../roadmap.md) and decisions in ADRs
[012](../adr/012-audio-output-via-cpal.md),
[013](../adr/013-compressed-audio-codecs.md) and
[014](../adr/014-streaming-audio-decode.md); this file only says *who owns
what* and *what is queued*. Update it in the same change that moves an
item.

## Owned crates

| Crate | Tier | Role |
|---|---|---|
| `meridian-audio-core` | 3 | Spatial audio on the GAC: `Listener`/`Emitter` frames, `SpeakerLayout` + 2D VBAP panning, `AttenuationModel`, `Mixer` (per-sample gains and block `render_interleaved`), `AudioOutput` bridge to the driver, `effects/` (`DspNode`/`DspGraph`, `Gain`, `LowPassFilter`, `Declicker`, `AcousticMedium`, `BinauralRenderer`). |
| `meridian-audio-driver` | 2 | `cpal`-backed device enumeration (`async`, ADR 009) and a push-model `AudioStream` over a bounded ring. No spatial or mixing knowledge. |

Co-owned, not owned:

- `meridian-asset-core::audio` (`wav`, `compressed_audio`,
  `audio_streaming`, `open_audio`) — owned by team/data; audio reviews
  changes that alter decoded sample layout, rates or the streaming
  `Iterator` contract.
- `meridian-engine-core::AudioSubsystem` and the audio half of
  `Runtime::tick` — owned by team/core; audio owns what `Mixer::mix`
  returns, not how the frame loop schedules it.
- `meridian-sdk::assets::load_audio_track` and the audio rig in
  `examples/magic_figures` — shared with team/core (SDK) and team/rendering
  (windowed example).

Owned docs: ADR 012/013/014 (013/014 jointly with team/data), the audio
paragraphs and the `meridian-audio-effects` entry of `roadmap.md`, this
file.

## Interfaces with other segments

- **team/core** — `gac-core` (`Motor3`, `Vec3`) for listener/emitter
  frames; `resource-core` handles; `engine-core` orchestration. Audio
  consumes these and never copies their logic.
- **team/physics** — emitter frames follow physics bodies only through
  `engine-core` (physics-then-audio in `Runtime::tick`); `AcousticMedium`
  is audio's, not a physics material.
- **team/data** — decoders and streaming live in `asset-core`; audio owns
  no decoders.
- **team/scripting** — `.mel` scenes reach audio only through
  app-defined `#[dsl_tag]` tags (`magic_figures`' `Audio` tag); the DSL
  stays a consumer of audio-core types, never a place for audio logic.
- **team/standards** — Craftbook rules apply once published; see
  "Standards gaps" below.

Skills to invoke here: `meridian-subsystem-core-keeper` (always),
`meridian-gac-math-keeper` (listener/emitter frames, azimuth, ITD
geometry), `meridian-crate-boundary-keeper` (any `Cargo.toml` edit),
`meridian-resource-lifetime-keeper` (anything touching `asset-core::audio`).

## Current state (2026-10-27)

Real and tested on CPU: VBAP panning for mono/stereo/headphones/5.0/5.1,
front-hemisphere fold for front-only layouts, inverse-distance
attenuation, `Mixer::mix`/`render_interleaved`, the effect chain and the
binaural renderer (ITD, head shadow, rear damping). Real on hardware:
`audio-driver` over `cpal`, driven end to end by `magic_figures`.
`cargo test -p meridian-audio-core`: 37 passing.

**Container note.** `audio-driver` needs ALSA headers to *build* on Linux
(`alsa-sys` fails without them): install `libasound2-dev` (Debian/Ubuntu)
first. Even then, in a container without a sound device 4 of 8
`audio-driver` tests fail (see backlog item 1); `audio-core` tests pass.

## Backlog

Not started; ordered by suggested priority. Items wait for an explicit task.

1. **Driver tests should skip, not fail, without a usable device.**
   `device_or_skip` only guards `AudioDevice::new`; in a container ALSA
   returns a device whose `supported_output_configs`/`open_stream` then
   fail, and the tests `unwrap()` them. Extend the skip to cover "device
   exists but cannot open a stream", so `cargo test --workspace` is green
   on CI and in containers without hiding real failures on a desktop.
2. **No panics on bad input.** `SpeakerLayout::pan` sorts and picks with
   `partial_cmp(..).unwrap()`; a NaN azimuth (emitter frame with NaN
   components) panics inside the mixer. Use `total_cmp` or reject
   non-finite azimuths, and add a test.
3. **Allocation-free hot path.** `pan`/`spatial_gains` allocate two `Vec`s
   and re-sort the speaker list per emitter per call, and `Mixer::mix`
   allocates its output per sample; `render_interleaved`/
   `BinauralRenderer::render` return fresh `Vec`s per block. Cache the
   sorted directional speakers in `SpeakerLayout` and add `*_into(&mut
   [f32])` variants that write into a caller buffer. Measure first.
4. **Split `audio-core/src/lib.rs` (937 lines).** Move layout/panning,
   attenuation, mixer and output into their own files behind the existing
   re-exports, matching the one-concern-per-file pattern `effects/`
   already uses. No API change.
5. **`docs/audio-design.md`.** Graphics and physics have design docs;
   audio's conventions (listener-local axes, VBAP edge rules, binaural
   model, latency/ring sizing) live only in module docs and the roadmap.
   Write a short design doc and point `architecture.md` at it.
6. **Emitter playback state.** `Emitter`'s doc says "frame + playback
   state" but it holds only `frame`; voices, looping, per-emitter gain and
   the link from a decoded/streamed track to an emitter are unmodelled.
   Design before code (ECS components vs audio-core voice pool), together
   with team/core.
7. **`magic_figures` through a `Stage`.** `engine-core` notes the
   binaural path still bypasses `Stage`; revisit once item 6 gives the
   stage a clean input shape.

Deliberately not queued: `meridian-audio-effects` (roadmap: only when an
effect needs an FFT/external crate), HRTF, height channels, a `Fixed`
flavor (audio output is not part of deterministic simulation; revisit only
if replay needs bit-exact audio).

## Standards gaps

Craftbook (team/standards) has no published rules yet. Once it does,
check audio against them; known candidates already visible: error types
(`AudioDeviceError` is the only typed error, `AudioOutput::open` just
forwards it), `unwrap()` in non-test code (item 2), file size/cohesion
(item 4), and test naming.
