# meridian-foundation

Zero-dependency foundational types shared workspace-wide: error conventions, unified logging, crash reporting, feature-detection primitives.

**Layer and context:** shared kernel · diagnostics

- **For:** `EngineError`, the process-wide log sink and `LogLevel`, `CrashReportConfig`.
- `FeatureFlags`: CPU feature detection shared by the compute crates.
- **Not for:** any subsystem concept; it depends on no other workspace crate.

## Usage

```rust
use meridian_foundation::FeatureFlags;

let flags = FeatureFlags::default();
assert!(!flags.simd128 && !flags.simd256);
```

## Features

| Feature | Enables |
|---|---|
| `file-logging` | Writes the log to a file (tokio-backed I/O) |

## Depends on / used by

- Depends on: no workspace crate
- Used by: [`meridian-asset-core`](../meridian-asset-core), [`meridian-audio-driver`](../meridian-audio-driver), [`meridian-dsl-core`](../meridian-dsl-core), [`meridian-engine-core`](../meridian-engine-core), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-numeric-core`](../meridian-numeric-core), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-platform-core`](../meridian-platform-core), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-core`](../meridian-ui-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
