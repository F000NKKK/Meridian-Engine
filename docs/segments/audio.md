# Segment: audio (`team/audio`)

Ownership note and working backlog for the audio segment. Status of record
stays in [roadmap.md](../roadmap.md) and decisions in ADRs
[012](../adr/012-audio-output-via-cpal.md),
[013](../adr/013-compressed-audio-codecs.md) and
[014](../adr/014-streaming-audio-decode.md); this file only says *who owns
what* and *what is queued*. Update it in the same change that moves an
item.

## Owned crates

Names follow [ADR 026](../adr/026-wave-7-crate-renames.md).

| Crate | Role |
|---|---|
| `meridian-audio-domain` | Spatial audio on the GAC: `Listener`/`Emitter` frames, `SpeakerLayout` + 2D VBAP panning, `AttenuationModel`, `Mixer` (`mix`, block `render_interleaved`), `spatial_gains`, `effects/` (`DspNode`/`DspGraph`, `Gain`, `LowPassFilter`, `Declicker`, `AcousticMedium`, `BinauralRenderer`). Depends on `meridian-gac` only. |
| `meridian-audio-app` | Ports and use cases: the `AudioSink` port, `AudioSubsystem`, and the `testing` feature (`RecordingSink` fake + `run_contract` suite for adapters). |
| `meridian-audio-cpal` | The `cpal` adapter: async device enumeration (ADR 009), push-model `AudioStream` over a bounded ring, `AudioSink` implementation. |

Co-owned, not owned:

- `meridian-asset-domain::audio` and `meridian-asset-symphonia` (decoders,
  streaming, `open_audio`) — team/data; audio reviews changes to decoded
  sample layout, rates or the streaming `Iterator` contract.
- `meridian-runtime-app` — team/core; audio owns what the mixer returns,
  not how the frame loop schedules it.
- `meridian-sdk::assets::load_audio_track` and the audio rig in
  `examples/magic_figures` — shared with team/core and team/rendering.

Owned docs: ADR 012/013/014 (013/014 jointly with team/data), the audio
parts of `roadmap.md`, this file. Earlier ADRs use pre-rename crate names.

## Interfaces with other segments

- **team/core** — `meridian-gac` frames, `meridian-resource` handles,
  `runtime-app` orchestration. Audio consumes these, never copies them.
- **team/physics** — emitter frames follow bodies only through
  `runtime-app`; `AcousticMedium` is audio's, not a physics material.
- **team/data** — decoders live in `asset-domain`; audio owns none.
- **team/scripting** — `.mel` scenes reach audio only via app-defined tags
  (`magic_figures`' `Audio` tag); the DSL never holds audio logic.
- **team/standards** — Craftbook rules apply (ADR 020).

Skills: `meridian-subsystem-core-keeper`, `meridian-gac-math-keeper`,
`meridian-crate-boundary-keeper` (any `Cargo.toml`),
`meridian-resource-lifetime-keeper` (asset audio).

## Current state (2026-10-27, after waves 5-7)

Done since the first draft of this note: the port/adapter split
(`AudioSink` in `audio-app`, `cpal` in `audio-cpal`); device-dependent
tests moved behind the `hw-tests` feature so CI and containers are green;
NaN-safe panning (`total_cmp`); `lib.rs` split one concern per file.
`cargo test -p meridian-audio-domain -p meridian-audio-app -p
meridian-audio-cpal` passes (36 + 3 + 5 tests). Building `audio-cpal` on
Linux still needs `libasound2-dev`.

## Backlog

Not started; ordered by suggested priority. Items wait for an explicit task.

1. ~~Allocation-free hot path~~ — done: `SpeakerLayout::pan_into`
   (no sort, no allocation), `spatial_gains_into`, `Mixer::mix_into` /
   `render_interleaved_into` (stack scratch up to 16 speakers) and
   `BinauralRenderer::render_into`; the allocating forms wrap them.
2. ~~Verify the NaN fix~~ — done: a test drives a NaN emitter position
   through the mixer.
3. ~~`docs/audio-design.md`~~ — done: [audio-design.md](../audio-design.md).
4. ~~Emitter playback state~~ — done: `Voice` (+ `SampleSource`,
   `MemorySource`, `PlaybackState`) in `audio-domain`, `AudioScene` in
   `audio-app`, `StreamSink` (a `Send` handle to the cpal ring) in
   `audio-cpal`.
5. ~~`magic_figures` through a `Stage`~~ — done: `AudioStage` in
   `runtime-app`; the example sets the listener and voice emitters and
   ticks.
6. ~~Voice features~~ — done: fades (`Voice::with_fade`), voice limit with
   stealing (`AudioScene::with_max_voices`), `ResampledSource`;
   `magic_figures` resamples tracks to the stream rate.
7. ~~Contact sounds~~ — done ([ADR 031](../adr/031-contact-events-through-the-event-system.md)):
   `ContactWatcher` in `physics-app`, `ContactSoundStage` in `runtime-app`.
8. ~~Priority stealing and filtered resampling~~ — done: `Voice::priority`
   (lowest first, oldest among equals) and `ResampledSource::anti_aliased`.
9. **Not built:** `AudioTrack` reads without the per-block `Vec`.

Deliberately not queued: `meridian-audio-effects` (only when an effect
needs an FFT/external crate), HRTF, height channels, a `Fixed` flavor
(revisit only if replay needs bit-exact audio).

## Standards gaps

Craftbook rules are adopted ([ADR 020](../adr/020-craftbook-adoption.md));
the port/adapter shape, `testing` feature and `hw-tests` already follow
them. Still to audit: typed errors beyond `AudioDeviceError`, remaining
`unwrap()` in non-test code, and the workspace size lints (`deny`).
