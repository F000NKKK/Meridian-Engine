# `meridian-audio-driver` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

**Stream model**: CPAL uses a callback-driven model — the audio
hardware calls a user-provided function on a real-time audio thread to
fill an output buffer. This crate bridges that into a push-based model
suitable for a game engine: [`AudioStream::push_samples`] is called
from the main/game thread to write interleaved float samples into an
internal ring buffer, and the CPAL callback drains that ring buffer.
This decouples the game's audio processing rate from the hardware's
buffer request rate — the game can push a full frame's worth of audio
at once, and the hardware drains it in smaller chunks.

**Async on genuine I/O**: [`AudioDevice::new`] is a real `async fn`
(enumerating audio devices is an OS/driver handshake with unbounded
completion time, matching the same policy as
`graphics-driver::Device::new` — see ADR 009). Stream creation and
sample pushing are synchronous (bounded, local work).
