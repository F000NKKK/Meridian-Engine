# meridian-sdk

Single application-facing entry point: re-exports everything an app needs (gac-core/physics-core/audio-core/graphics-core/graphics-driver/platform-core/asset-core/engine-core types) plus its own composable job-graph pipeline and windowed-app scaffolding (scene/camera helpers). Applications depend on this crate alone, never on the individual crates it composes.

**Layer and context:** interface · runtime

- **For:** The single application entry point: re-exports the other crates' types, plus the scene DSL (`dsl`), `mel_include!`, `RenderStage` and windowed-app scaffolding.
- **Not for:** implementing a subsystem: applications depend on this crate alone and never on the crates it composes.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_sdk::dsl;

let registry = dsl::default_registry();
```

## Features

| Feature | Enables |
|---|---|
| `file-logging` | File log sink (forwards to `meridian-foundation`) |

## Depends on / used by

- Depends on: [`meridian-asset-core`](../meridian-asset-core), [`meridian-asset-png`](../meridian-asset-png), [`meridian-asset-symphonia`](../meridian-asset-symphonia), [`meridian-audio-app`](../meridian-audio-app), [`meridian-audio-core`](../meridian-audio-core), [`meridian-audio-driver`](../meridian-audio-driver), [`meridian-compute-driver`](../meridian-compute-driver), [`meridian-compute-runtime`](../meridian-compute-runtime), [`meridian-dsl-core`](../meridian-dsl-core), [`meridian-dsl-macros`](../meridian-dsl-macros), [`meridian-engine-core`](../meridian-engine-core), [`meridian-foundation`](../meridian-foundation), [`meridian-gac-core`](../meridian-gac-core), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-physics-core`](../meridian-physics-core), [`meridian-platform-core`](../meridian-platform-core), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-ui-core`](../meridian-ui-core)
- Used by: [`meridian-examples`](../../examples)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
