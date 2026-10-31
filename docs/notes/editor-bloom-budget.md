# Editor bloom frame-budget comparison

A non-emissive scene previously still ran the emissive redraw, four fullscreen blur passes
and additive composition every frame. The prepared scene now checks the emissive factors
of its referenced materials and skips this work when they are all zero. A real GPU test
switches plain → emissive → plain materials, checks the pass labels when timestamp queries
are supported, and compares the resulting pixels to guard against stale glow.

## Paired local benchmark

Both runs use the same `editor_frame_budget` test code and Rust 1.99 on Linux with
llvmpipe (LLVM 19.1.7, 256 bits), GL backend. The baseline resolves `meridian-sdk = "=0.7.0"`
from crates.io; the changed run resolves the local workspace. Scene: one bound builtin sphere,
ground grid and moving camera, 1280×720, 10 warm-up frames and 120 recorded frames.
Timings include engine rendering, RGBA readback and a copy into a wire buffer; they exclude
Tauri IPC and webview presentation. GPU profiling is off for these samples.

| Version | Frame p50 | Frame p95 | Frame p99 |
| --- | ---: | ---: | ---: |
| Published SDK 0.7.0 | 221.40 ms | 273.48 ms | 287.46 ms |
| Empty bloom skipped | 41.94 ms | 61.60 ms | 69.86 ms |

The improvement is roughly 5.3× at p50 on this software adapter. It does not establish
120 FPS: the target is 8.33 ms, and the result is not a physical GPU measurement.

Run the changed benchmark with:

```sh
cargo test -p meridian-sdk --test editor_frame_budget -- --ignored --nocapture
```

The new test and optimization are subsequent source work; they are not in SDK 0.7.0 or IDLE 0.2.0.
