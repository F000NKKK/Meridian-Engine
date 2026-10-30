# Meridian SDK 0.7.0

Applications can register authored rendering components and extract a 3D scene using only
`meridian-sdk`: `persistence::register_rendering` and `extract_scene3d` are now public facade exports.
This supports CLI-generated games that load their startup scene from project data and compose generated MEUI.

- Includes `meridian-platform-winit` 0.12.1 with F3 input for diagnostics overlays.
- Adds a downstream regression test for loading an authored scene through the SDK facade.
- Existing subsystem crates retain their published versions when unchanged.

The source archive is the SDK checkout. The examples and sample assets are attached separately.
Project creation, `.mproj` processing and game templates are supplied by Meridian CLI 0.2.0.
The IDLE viewport performance work is still in progress; this release makes no 120 FPS guarantee.
