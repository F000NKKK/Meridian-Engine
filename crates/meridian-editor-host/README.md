# meridian-editor-host

The game side of the editor protocol, without I/O: a `Session` that answers an editor's requests against a `World` and a `ComponentRegistry`, and the events they cause.

**Layer and context:** infrastructure (adapter) · editor

- **For:** The handshake, `Describe`, entity and component requests, play control and event subscriptions of [`meridian-editor-protocol`](../meridian-editor-protocol), as a synchronous state machine. The simulation is driven through the `SimulationControl` port.
- **Not for:** sockets or threads (that is [`meridian-editor-tcp`](../meridian-editor-tcp)), or stepping the simulation (that is `engine-core`'s frame loop).

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-editor-host); see `src/lib.rs` for the module overview and `docs/editor-protocol.md` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-editor-protocol`](../meridian-editor-protocol), [`meridian-scene`](../meridian-scene)
- Used by: [`meridian-editor-tcp`](../meridian-editor-tcp)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
