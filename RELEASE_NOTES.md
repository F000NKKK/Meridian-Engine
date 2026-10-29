# Meridian SDK 0.6.0

The current modular engine SDK, published together with its dependency crates after the
Craftbook crate/layer migration. Applications use `meridian-sdk` as their entry point.

- Domain/application/backend crate boundaries and typed compute dispatch.
- Scene/prefab persistence on Substrate Data and editor protocol support.
- Editor rendering, asset binding, picking and scene overlays.
- Rendering, physics, audio, animation and retained UI composition through the SDK.

This is a breaking 0.x minor release: crate names and subsystem APIs have changed since 0.5.0.
The source archive is the SDK checkout. `meridian-examples.zip` and `meridian-assets.zip`
provide the examples and sample content consumed by Meridian Installer. Rust 1.99 is required.
The proposed `.mproj` format is not implemented in this release.
