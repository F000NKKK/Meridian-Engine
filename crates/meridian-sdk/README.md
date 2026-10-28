# meridian-sdk

Single application-facing entry point: re-exports everything an app needs (gac/physics-domain/audio-domain/rendering-domain/rendering-wgpu/platform-winit/asset-domain/runtime-app types) plus its own composable job-graph pipeline and windowed-app scaffolding (scene/camera helpers). Applications depend on this crate alone, never on the individual crates it composes.

**Layer and context:** interface · runtime

- **For:** The single application entry point: re-exports the other crates' types, plus the scene DSL (`dsl`), `meui_include!`, `RenderStage` and windowed-app scaffolding.
- **Not for:** implementing a subsystem: applications depend on this crate alone and never on the crates it composes.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_sdk::dsl;

let registry = dsl::default_registry();
```

## Depends on / used by

- Depends on: [`meridian-data-substrate`](../meridian-data-substrate), [`meridian-ecs`](../meridian-ecs), [`meridian-asset-domain`](../meridian-asset-domain), [`meridian-asset-gltf`](../meridian-asset-gltf), [`meridian-asset-jpeg`](../meridian-asset-jpeg), [`meridian-asset-png`](../meridian-asset-png), [`meridian-asset-symphonia`](../meridian-asset-symphonia), [`meridian-audio-app`](../meridian-audio-app), [`meridian-audio-domain`](../meridian-audio-domain), [`meridian-audio-cpal`](../meridian-audio-cpal), [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-compute-app`](../meridian-compute-app), [`meridian-diagnostics-fs`](../meridian-diagnostics-fs), [`meridian-markup`](../meridian-markup), [`meridian-markup-macros`](../meridian-markup-macros), [`meridian-runtime-app`](../meridian-runtime-app), [`meridian-diagnostics`](../meridian-diagnostics), [`meridian-gac`](../meridian-gac), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-physics-app`](../meridian-physics-app), [`meridian-physics-wgpu`](../meridian-physics-wgpu), [`meridian-physics-domain`](../meridian-physics-domain), [`meridian-platform-winit`](../meridian-platform-winit), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-runtime-threads`](../meridian-runtime-threads), [`meridian-ui-domain`](../meridian-ui-domain)
- Used by: [`meridian-examples`](../../examples)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
