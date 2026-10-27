# `meridian-rendering-wgpu` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

**Async on genuine I/O, not on everything** — see
[ADR 009](../adr/009-async-io-via-tokio.md) and
`meridian_gpu_wgpu`'s module doc for the same policy this crate
inherits: [`Device::new`]/[`Device::new_windowed`]/[`Device::read_buffer`]
are real `async fn`s; recording/allocation calls stay synchronous.

Everything here is exercised end-to-end by the `magic_figures`/
`physic_figures` examples (windowed render path); `meridian_gpu_wgpu`'s
own tests cover the shared device/buffer/shader mechanics (headless
compute path).
