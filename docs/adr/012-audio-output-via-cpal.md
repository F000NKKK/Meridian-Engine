# ADR 012: Audio output via cpal, not hand-written per-platform FFI

## Status

Accepted — supersedes the "audio-driver is still a scaffold, no
ecosystem-standard crate has been evaluated" status recorded in earlier
`docs/roadmap.md` revisions.

## Context

`meridian-audio-driver` was a deliberate scaffold while `audio-core`
(panning, attenuation, mixing, DSP — all driver-independent) was built
and validated first. Making the driver real means talking to an actual
OS audio output device, which is the same shape of problem
[ADR 010](010-windowing-via-winit.md) documented for windowing: correct
cross-platform audio output is not "open a device and write samples" —
it's backend negotiation (ALSA vs. PulseAudio vs. PipeWire on Linux
alone), device enumeration and hotplug, sample-format/rate negotiation,
and a real-time callback thread with its own scheduling and underrun
semantics, per platform (WASAPI on Windows, CoreAudio on macOS). Doing
that by hand is a multi-month undertaking with three or more independent
classes of platform bugs.

## Decision

**`cpal`, not hand-written per-platform FFI.** `cpal` is to audio output
what `winit` is to windowing and `wgpu` is to GPU access: the de facto
standard Rust crate for exactly this OS boundary, wrapping
ALSA/WASAPI/CoreAudio (and more) behind one device/stream API. The full
rationale lives on the workspace `Cargo.toml`'s `cpal` entry.

**The driver bridges cpal's callback model into a push model.** cpal is
callback-driven (the hardware calls into user code on a real-time audio
thread). `audio-driver` exposes a push API instead —
`AudioStream::push_samples` writes interleaved `f32` samples into a
bounded ring buffer (a `crossbeam-channel` bounded channel) that the
cpal callback drains, with blocking-on-full as the backpressure
mechanism. This decouples the game's audio production rate from the
hardware's buffer request rate, which is the model `engine-core`'s frame
loop actually needs.

**Driver/core separation is unchanged** ([ADR 005](005-driver-core-separation.md)):
`audio-driver` knows devices, streams, and sample transport;
`audio-core` keeps panning/attenuation/mixing/DSP and never appears in
the driver's dependency graph.

**`AudioDevice::new` is `async`; everything else is synchronous**
([ADR 009](009-async-io-via-tokio.md)): device enumeration is an
OS/driver handshake (genuine I/O, same precedent as
`graphics-driver::Device::new`); stream creation and sample pushing are
bounded local work and stay plain functions.

## Alternatives considered

- **Hand-written per-platform FFI** — rejected for the same
  scope-on-closer-inspection reason as ADR 010; see "Context".
- **SDL2's audio subsystem** — rejected: same reasoning as in ADR 010's
  alternatives — this workspace has no other reason to adopt SDL's
  broader scope, and `cpal` is the audio-focused equivalent already
  standard in the Rust ecosystem (rodio, bevy_audio, and most Rust
  engines sit on it).
- **PipeWire/PulseAudio bindings directly (Linux-only first)** —
  rejected: buys a single platform for comparable effort to integrating
  `cpal`, which buys all of them.

## Consequences

- `audio-driver` gains two external dependencies (`cpal`,
  `crossbeam-channel`) — the same kind of deliberate, documented
  exception to zero-deps that `wgpu` and `winit` established.
- Only the `f32` sample format is supported initially; other formats
  return `UnsupportedConfig` and can be added when a real device demands
  one, not speculatively.
- A real audible example is unblocked; see [roadmap.md](../roadmap.md)
  for current status.
