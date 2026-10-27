# meridian-scene

Scene serialization on Substrate Data: an application-built component registry, the .mscene document schema, and World load/save with line/column/path diagnostics.

**Layer and context:** infrastructure (adapter) · data

- **For:** The `.mscene` document schema on Substrate Data, an application-built `ComponentRegistry`, and `World` load/save with line/column/path diagnostics.
- **Not for:** the editor wire protocol: that is `meridian-editor-protocol`. The `meridian-sdk` does not re-export it yet.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-scene); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-gac-core`](../meridian-gac-core)
- Used by: [`meridian-editor-protocol`](../meridian-editor-protocol)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
