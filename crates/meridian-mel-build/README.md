# meridian-mel-build

Build-time .meui generator: turns each .meui document into a generated <name>.g.rs (x:Name ids, typed builder, code-behind aliases) that a code-behind pulls in with meridian_sdk::mel_include!. Used from build.rs; see ADR 017.

**Layer and context:** infrastructure (adapter) · ui

- **For:** `compile_dir`/`compile_file` for a `build.rs`: each `.meui` becomes a generated `<name>.g.rs`.
- **Not for:** parsing at run time: use `meridian-dsl-core` or `meridian_sdk::dsl`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
// build.rs
fn main() {
    if let Err(error) = meridian_mel_build::compile_dir("assets/scenes") {
        panic!("{error}");
    }
}
```

## Depends on / used by

- Depends on: [`meridian-dsl-core`](../meridian-dsl-core), [`meridian-ui-core`](../meridian-ui-core)
- Used by: no workspace crate

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
