# meridian-ui-domain

WPF-style retained UI runtime: element tree, typed dependency properties, measure/arrange layout (StackPanel, DockPanel, Grid, ScrollViewer) and hit testing. Emits no pixels itself; see ADR 018.

**Layer and context:** domain · ui

- **For:** A WPF-style retained UI runtime: `UiTree`, typed properties, measure/arrange layout and hit testing.
- **Not for:** pixels: it renders nothing itself; extraction to `Scene2d` lives in `meridian-ui-scene2d`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-ui-domain); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics), [`meridian-memory`](../meridian-memory)
- Used by: [`meridian-ui-mel`](../meridian-ui-mel), [`meridian-sdk`](../meridian-sdk), [`meridian-ui-scene2d`](../meridian-ui-scene2d)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
