# meridian-ui-mel

Build-time .meui generator: turns each .meui document into a generated <name>.g.rs (x:Name ids, typed builder, code-behind aliases) that a code-behind pulls in with meridian_sdk::meui_include!. Used from build.rs; see ADR 017.

**Layer and context:** infrastructure (adapter) · ui

- **For:** `compile_dir`/`compile_file` for a `build.rs`: each `.meui` becomes a generated `<name>.g.rs`.
- **Not for:** parsing at run time: use `meridian-markup` or `meridian_sdk::dsl`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
// build.rs
fn main() {
    if let Err(error) = meridian_ui_mel::compile_dir("assets/scenes") {
        panic!("{error}");
    }
}
```

## Depends on / used by

- Depends on: [`meridian-markup`](../meridian-markup), [`meridian-ui-domain`](../meridian-ui-domain)
- Used by: no workspace crate

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
