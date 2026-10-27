# meridian-data-substrate

Scene serialization on Substrate Data: an application-built component registry, the .mscene document schema, and World load/save with line/column/path diagnostics.

**Layer and context:** infrastructure (adapter) · data

- **For:** The `.mscene` document schema on Substrate Data, an application-built `ComponentRegistry`, and `World` load/save with line/column/path diagnostics.
- **Not for:** the editor wire protocol: that is `meridian-editor-domain`. The `meridian-sdk` does not re-export it yet.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-data-substrate); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-ecs`](../meridian-ecs), [`meridian-gac`](../meridian-gac)
- Used by: [`meridian-editor-app`](../meridian-editor-app), [`meridian-editor-domain`](../meridian-editor-domain)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
