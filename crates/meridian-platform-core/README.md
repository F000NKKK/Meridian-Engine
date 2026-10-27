# meridian-platform-core

OS abstraction: window, input, filesystem, time, threading and dynamic library loading.

**Layer and context:** infrastructure (adapter) · platform

- **For:** The OS window and event loop (`winit`), input state, `Clock`, CPU/GPU capability reports, dynamic libraries.
- **Not for:** GPU devices and audio devices: those are `meridian-gpu-driver` and `meridian-audio-driver`.

## Usage

```rust
use meridian_platform_core::Clock;

let mut clock = Clock::new();
let time = clock.tick();
assert!(time.delta_seconds >= 0.0);
```

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation)
- Used by: [`meridian-asset-core`](../meridian-asset-core), [`meridian-audio-driver`](../meridian-audio-driver), [`meridian-compute-driver`](../meridian-compute-driver), [`meridian-engine-core`](../meridian-engine-core), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-physics-driver`](../meridian-physics-driver), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
