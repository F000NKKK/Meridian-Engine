# meridian-audio-app

Audio use cases and the ports to the outside world: the `AudioSink` mixed samples leave through, and the `AudioSubsystem` state the runtime mixes each frame.

**Layer and context:** application · audio

- **For:** the plain-data `AudioSubsystem` (listener, emitters, mixer) and the `AudioSink` port (declared by the layer that needs it, ADR 019), with a `RecordingSink` fake and the `run_contract` suite under the `testing` feature.
- **Not for:** the mixer, DSP or spatialization: those are `meridian-audio-core`. Opening a device is `meridian-audio-driver`, which implements the port.

## Usage

```rust
use meridian_audio_app::AudioSink;

struct Silence;

impl AudioSink for Silence {
    fn push_interleaved(&self, _samples: &[f32]) {}
    fn can_push(&self, _sample_count: usize) -> bool {
        true
    }
}

assert!(Silence.can_push(64));
```

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::RecordingSink` and `testing::run_contract` for adapters' tests |

## Depends on / used by

- Depends on: [`meridian-audio-core`](../meridian-audio-core)
- Used by: [`meridian-audio-driver`](../meridian-audio-driver), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
