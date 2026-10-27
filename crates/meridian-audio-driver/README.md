# meridian-audio-driver

Low-level audio device abstraction: output streams, buffers and platform audio backends.

**Layer and context:** infrastructure (adapter) · audio

- **For:** The audio output device (via `cpal`): streams and ring buffers.
- **Not for:** mixing, DSP or spatialization: those are `meridian-audio-core`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-audio-driver); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation), [`meridian-platform-core`](../meridian-platform-core)
- Used by: [`meridian-audio-core`](../meridian-audio-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
