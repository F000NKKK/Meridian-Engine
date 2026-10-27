# meridian-diagnostics

Zero-dependency foundational types shared workspace-wide: error conventions, the log kernel and its `LogSink` port, feature-detection primitives.

**Layer and context:** shared kernel · diagnostics

- **For:** `EngineError`, the process-wide log (`log_*!`, `LogLevel`, the `LogSink` port); file output and crash reports are [`meridian-diagnostics-fs`](../meridian-diagnostics-fs).
- `FeatureFlags`: CPU feature detection shared by the compute crates.
- **Not for:** any subsystem concept; it depends on no other workspace crate.

## Usage

```rust
use meridian_diagnostics::FeatureFlags;

let flags = FeatureFlags::default();
assert!(!flags.simd128 && !flags.simd256);
```

## Depends on / used by

- Depends on: no workspace crate
- Used by: [`meridian-asset-domain`](../meridian-asset-domain), [`meridian-audio-cpal`](../meridian-audio-cpal), [`meridian-markup`](../meridian-markup), [`meridian-runtime-app`](../meridian-runtime-app), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-numeric`](../meridian-numeric), [`meridian-physics-wgpu`](../meridian-physics-wgpu), [`meridian-platform-winit`](../meridian-platform-winit), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-domain`](../meridian-ui-domain)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
