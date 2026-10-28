# meridian-data-substrate

Scene serialization on Substrate Data: an application-built component registry, the .mscene (text and binary), .mprefab and .msave formats, and World load/save with path and line/column or byte-offset diagnostics.

**Layer and context:** infrastructure (adapter) · data

- **For:** The `.mscene` (text and binary), `.mprefab` and `.msave` formats on Substrate Data, an application-built `ComponentRegistry`, and `World` load/save with path and line/column or byte-offset diagnostics. A world's disk form is its scene document; `World::snapshot` in `meridian-ecs` is the in-memory copy and is never written.
- **Not for:** the editor wire protocol (that is `meridian-editor-domain`), reading files (prefab bytes come through the `PrefabSource` port the application implements), or the game's own save `state` schema (the game owns it). The `meridian-sdk` does not re-export it yet.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-data-substrate); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-ecs`](../meridian-ecs), [`meridian-gac`](../meridian-gac)
- Used by: [`meridian-editor-app`](../meridian-editor-app), [`meridian-editor-domain`](../meridian-editor-domain)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
