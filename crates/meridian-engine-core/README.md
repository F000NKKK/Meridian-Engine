# meridian-engine-core

Runtime: frame scheduler, event system, subsystem manager; ties every other crate into the main loop.

**Layer and context:** application · runtime

- **For:** `Runtime`: frame scheduler, event system, physics and audio subsystems, and `Stage`s run on a `JobGraph`.
- **Not for:** subsystem algorithms: it only coordinates other crates.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-engine-core); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-audio-core`](../meridian-audio-core), [`meridian-compute-runtime`](../meridian-compute-runtime), [`meridian-ecs-core`](../meridian-ecs-core), [`meridian-foundation`](../meridian-foundation), [`meridian-gac-core`](../meridian-gac-core), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-physics-core`](../meridian-physics-core), [`meridian-platform-core`](../meridian-platform-core), [`meridian-task-core`](../meridian-task-core)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
