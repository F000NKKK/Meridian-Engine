# meridian-editor-tcp

Serves the editor protocol to one editor over a local TCP connection: Substrate Data frames in and out, a `RequestHandler` in the middle.

**Layer and context:** infrastructure (adapter) · editor

- **For:** Binding `127.0.0.1`, accepting one editor, and running the request/response loop of [`meridian-editor-app`](../meridian-editor-app) over any async byte stream. The only crate of the editor host that depends on `tokio`.
- **Not for:** answering requests (that is the `RequestHandler` you pass), authentication, or non-local connections: the listener binds loopback only.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-editor-tcp); see `src/lib.rs` for the module overview and `docs/editor-protocol.md` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-editor-app`](../meridian-editor-app), [`meridian-editor-domain`](../meridian-editor-domain)
- Used by: no workspace crate

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
