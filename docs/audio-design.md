# Audio design — `meridian-audio-domain`, `-app`, `-cpal`

## The split

| Crate | Layer | Owns |
|---|---|---|
| `meridian-audio-domain` | domain | Listener/emitter frames, panning, attenuation, mixing, effects, binaural rendering, `Voice` and its `SampleSource` port. Pure CPU math on `meridian-gac`; no I/O, no `async`. |
| `meridian-audio-app` | application | The `AudioSink` port (where mixed samples go), `AudioScene` (listener + voices + renderer + post effects) and `AudioSubsystem` (the gain-query path), plus the `testing` fake and contract suite. |
| `meridian-audio-cpal` | adapter | `cpal` device enumeration (`async`, [ADR 009](adr/009-async-io-via-tokio.md)), `AudioStream`, and `StreamSink`, its `Send` handle. Implements `AudioSink`. |

`runtime-app::AudioStage` is the frame-loop glue: it renders a shared
`AudioScene` into a sink. Decoding lives in `asset-domain`/`asset-symphonia`
([ADR 013](adr/013-compressed-audio-codecs.md),
[ADR 014](adr/014-streaming-audio-decode.md)); the SDK adapts a decoded track
to `SampleSource`. Ports sit with the layer that needs them
([ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md)).

## Spatial conventions

Listener-local axes are chosen here, not inherited: facing is `+X`, up is
`+Y`, right is `+Z` (`+X × +Y = +Z`). A listener at `Motor3::identity()` faces
world `+X`. An emitter's position is its frame applied to the origin; its
azimuth is `atan2(z, x)` of that position in listener space, `0°` ahead and
positive to the right. Elevation is ignored: there are no height channels.

## Panning

Horizontal-plane 2D VBAP with constant power. A source azimuth is bracketed
by the two adjacent directional speakers and their gains are
`cos(t·90°)`/`sin(t·90°)`, so the squared gains always sum to one. The LFE
channel (no azimuth) never receives directional gain.

- **Wrapping layouts** (5.0/5.1) pan around the full ring; front and back differ.
- **Front-only layouts** (mono, stereo, headphones) first fold the azimuth into
  `[-90°, 90°]` (`fold_to_front_hemisphere`): ahead and behind are
  indistinguishable to amplitude panning, so they must produce the same
  centred pan. Beyond the outermost speaker the source clamps to that speaker.
- A single directional speaker gets gain `1.0`; none gives silence.

`SpeakerLayout::pan_into` is allocation-free and sorts nothing: it finds the
bracketing speakers by one scan. A non-finite azimuth (an emitter
frame with NaN components) renders silence, never NaN gains or a panic.

## Attenuation

`AttenuationModel` is OpenAL's inverse-clamped-distance model: gain `1.0` at or
inside `reference_distance`, falling as `ref / (ref + rolloff·(d − ref))`, with
`d` clamped to `max_distance` so a far source never reaches exactly zero.

## Rendering

Two renderers sit behind the `BlockRenderer` port; the composition root picks:

- **`Mixer`** (stateless): per-emitter gains once per block (poses do not move
  mid-block), then sample-wise weighted sums into interleaved speaker order.
  `mix_into` and `render_interleaved_into` write into caller buffers; per-emitter
  gains live on the stack up to 16 speakers.
- **`BinauralRenderer`** (stateful, stereo headphones): interaural time
  difference from a fractional delay line per source, head-shadow as a one-pole
  low-pass per ear, extra gain and cutoff reduction behind the listener,
  distance attenuation, and linear ramping of every parameter across a block so
  pose changes do not click. `AcousticMedium` supplies the speed of sound. It
  has no measured HRTF (that needs an impulse-response database) and keeps
  delay lines across calls, so call it once per block with sources in a stable
  order.

Post effects (`DspNode`/`DspGraph`: `Gain`, `LowPassFilter`, `Declicker`) run in
place over the interleaved block. Heavy effects that need an FFT crate would
live in a separate `meridian-audio-effects` crate, created only when one is
needed (see roadmap, "Not yet decided").

## Voices and the scene

A `Voice` is an `Emitter` frame, a gain and a `SampleSource`, with a
`PlaybackState` (`Playing`, `Paused`, `Finished`). The application moves the
emitter; the voice owns only playback. A source that returns fewer samples than
asked has ended: the block is zero-padded and the voice finishes. A paused voice
is silent and keeps its position. `MemorySource` plays a decoded buffer once or
looping (`Repeat`). `ResampledSource` converts a source recorded at another
rate to the stream's by linear interpolation (no low-pass before decimating, so
large downward conversions alias).

Fades: `Voice::with_fade(frames)` fades in at start and out on pause and stop
instead of cutting the waveform. `state()` changes at once; `is_done()` turns
true only when a stopped voice is silent, and scenes reap on `is_done`. A block
that contains the end of a fade-out still consumes a full block of source
samples.

`AudioScene` holds the listener, the voices (addressed by `VoiceId`, never
reused), one `BlockRenderer` and a post-effect chain. `render_block` advances
every voice by the same frame count, spatializes them and applies the effects.
`with_max_voices` caps the scene: adding a voice to a full scene drops done
voices first, then steals the oldest. It is plain data; the lock is
`runtime-app::Shared`.

## Output and latency

`AudioStage::run` tops the sink up with whole blocks of `block_frames` and stops
at the first block the sink has no room for, so it never blocks a frame (a
per-run cap guards a sink that always has room). The sink's ring is the
latency: audio mixed against a listener pose waits in it until the hardware
drains it. `magic_figures` uses 10 ms blocks and an 80 ms ring.

`cpal::Stream` is not `Send` on every platform, so `AudioStream` stays on the
thread that opened it and `AudioStream::sink()` hands the stage a `StreamSink`
that shares only the sample ring.

## Determinism

Audio output is not part of the deterministic simulation, so audio has no
`Fixed` flavor. If replays ever need bit-exact audio, revisit with
[ADR 008](adr/008-fixed-point-determinism.md).

## Testing

Domain behaviour is tested on CPU (constant power around the ring, fold, clamp,
binaural level and delay differences, voice state transitions). Sink
implementations run `audio-app::testing::run_contract`. Tests that need a real
device are behind the `hw-tests` feature.
