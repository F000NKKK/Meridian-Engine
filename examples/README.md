# meridian-examples

Example programs for the engine, plus the small shared plumbing they use. Not published.

**Layer and context:** interface · runtime

| Example | What it shows | Run |
|---|---|---|
| `magic_figures` | Three orbiting, textured, glowing shapes, each playing its own spatialized music track, with bloom. The scene comes from `assets/scenes/magic_figures.meui`, compiled by `build.rs` through `meridian-ui-mel`. | `./build.sh run magic_figures` |
| `gltf_viewer` | Loads a `.glb` or `.gltf` given on the command line and orbits a camera around it (an optional second argument sets the distance in model radii, below 1 for the inside of a scene). The models are not in the repository: the Khronos glTF-Sample-Assets work. | `./build.sh run gltf_viewer -- <model>` |
| `physic_figures` | The same shapes as rigid bodies falling onto a textured floor, driven through `runtime-app`'s `Runtime`. The scene file is read at run time. | `./build.sh run physic_figures` |

Both open a window, so they need a display and a GPU adapter. They import everything through [`meridian-sdk`](../crates/meridian-sdk).

## License

MPL-2.0, see the repository [LICENSE](../LICENSE).
