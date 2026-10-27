# meridian-examples

Example programs for the engine, plus the small shared plumbing they use. Not published.

**Layer and context:** interface · runtime

| Example | What it shows | Run |
|---|---|---|
| `magic_figures` | Three orbiting, textured, glowing shapes, each playing its own spatialized music track, with bloom. The scene comes from `assets/scenes/magic_figures.meui`, compiled by `build.rs` through `meridian-ui-mel`. | `./build.sh run magic_figures` |
| `physic_figures` | The same shapes as rigid bodies falling onto a textured floor, driven through `runtime-app`'s `Runtime`. The scene file is read at run time. | `./build.sh run physic_figures` |

Both open a window, so they need a display and a GPU adapter. They import everything through [`meridian-sdk`](../crates/meridian-sdk).

## License

MPL-2.0, see the repository [LICENSE](../LICENSE).
