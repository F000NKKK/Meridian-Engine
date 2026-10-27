# meridian-ui-core

WPF-style retained UI runtime: element tree, typed dependency properties, measure/arrange layout (StackPanel, DockPanel, Grid) and hit testing. Emits no pixels itself; see ADR 018.

**Layer and context:** domain · ui

- **For:** A WPF-style retained UI runtime: `UiTree`, typed properties, measure/arrange layout and hit testing.
- **Not for:** pixels: it renders nothing itself; extraction to `Scene2d` and text drawing are not done yet (see ADR 018).

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-ui-core); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation), [`meridian-memory-core`](../meridian-memory-core)
- Used by: [`meridian-mel-build`](../meridian-mel-build), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-render`](../meridian-ui-render)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
