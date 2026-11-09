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

## Window shutdown

`AppHandler::on_close_requested` accepts or rejects an OS close request.
After acceptance, `ready_to_close` may keep the event loop alive until the
application's rendering and background workers have finished. No further
redraw callbacks run during this shutdown interval. The default readiness
returns true and preserves immediate-close behavior for simple clients.
Both the main Enter key and numeric-keypad Enter map to `KeyCode::Enter`.

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics)
- Used by: [`meridian-audio-cpal`](../meridian-audio-cpal), [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
