# meridian-editor-protocol

The messages between a running Meridian game and its editor (Meridian IDLE): typed requests, responses and events as Substrate Data frames, append-only.

**Layer and context:** infrastructure (adapter) · editor

- **For:** Typed, append-only requests, responses and events between a running game and its editor, framed as Substrate Data (`read_message`/`write_message`).
- **Not for:** storing scenes: that is `meridian-scene`. The host that answers requests is [`meridian-editor-host`](../meridian-editor-host).

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-editor-protocol); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-scene`](../meridian-scene)
- Used by: [`meridian-editor-host`](../meridian-editor-host), [`meridian-editor-tcp`](../meridian-editor-tcp)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
