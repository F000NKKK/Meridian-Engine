# meridian-dsl-macros

#[dsl_tag] attribute macro implementing meridian_dsl_core::DslTag for a struct — the code-generation half of the SDK's extensible scene DSL; see meridian-dsl-core for the trait and meridian-sdk::dsl for the registry it feeds.

**Layer and context:** interface · scene-markup

- **For:** The `#[dsl_tag(name = "...")]` attribute macro that implements `DslTag` for a struct.
- **Not for:** parsing or the registry: those are `meridian-dsl-core`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_sdk::dsl::dsl_tag;

#[dsl_tag(name = "Glow")]
struct Glow {
    r: f32,
    g: f32,
    b: f32,
}
```

## Depends on / used by

- Depends on: no workspace crate
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
