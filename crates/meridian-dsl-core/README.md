# meridian-dsl-core

Generic tag/attribute/children text parser underlying the SDK's extensible scene DSL — no knowledge of any domain tag (`Mesh`, `RigidBody`, ...); see `meridian-sdk::dsl` for that layer.

**Layer and context:** shared kernel · scene-markup

- **For:** A domain-blind `.meui` tag/attribute parser: `parse`, spans for diagnostics, markup extensions, `DslRegistry` and the `DslTag` trait.
- **Not for:** knowing any tag: `Mesh`, `RigidBody` and the rest live in `meridian-sdk::dsl`; the `#[dsl_tag]` macro is `meridian-dsl-macros`.

## Usage

```rust
use meridian_dsl_core::parse;

let root = parse(r#"<Scene><Entity x:Name="cube" /></Scene>"#).unwrap();
assert_eq!(root.name.local(), "Scene");
assert_eq!(root.children.len(), 1);
```

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation)
- Used by: [`meridian-mel-build`](../meridian-mel-build), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
