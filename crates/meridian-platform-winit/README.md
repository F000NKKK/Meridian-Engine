# meridian-platform-winit

OS abstraction: window, input, filesystem, time, threading and dynamic library loading.

**Layer and context:** infrastructure (adapter) · platform

- **For:** The OS window and event loop (`winit`), input state, `Clock`, CPU/GPU capability reports, dynamic libraries.
- **Not for:** GPU devices and audio devices: those are `meridian-gpu-wgpu` and `meridian-audio-cpal`.

## Usage

```rust
use meridian_platform_winit::Clock;

let mut clock = Clock::new();
let time = clock.tick();
assert!(time.delta_seconds >= 0.0);
```

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics)
- Used by: [`meridian-audio-cpal`](../meridian-audio-cpal), [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
