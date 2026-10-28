# ADR 026: Wave 7 — crate names follow `<product>-<context>-<layer>`

## Status

Accepted. Carries out [ADR 020](020-craftbook-adoption.md) D4. Earlier ADRs
keep the names that were current when they were written; this table maps them.

## Decision

Every crate is renamed to `meridian-<context>-<layer>`, where the layer is
`domain`, `app`, or the adapter's technology (`wgpu`, `cpal`, `winit`, `fs`,
`threads`). The kernels drop the suffix (`meridian-gac`, `meridian-ecs`).

| Old | New |
|---|---|
| `foundation` | `diagnostics` (kernel: `EngineError`, log kernel, `LogSink`) |
| `numeric-core`, `gac-core`, `memory-core`, `resource-core`, `ecs-core` | `numeric`, `gac`, `memory`, `resource`, `ecs` |
| `task-core` | `runtime-threads` |
| `platform-core` | `platform-winit` (the input-state split into a kernel crate is still pending) |
| `gpu-driver`, `compute-driver`, `gac-compute` | `gpu-wgpu`, `compute-wgpu`, `gac-wgpu` |
| `compute-runtime` | `compute-app` |
| `asset-core` | `asset-domain` (`asset-png`, `asset-symphonia` keep their names) |
| `graphics-core`, `graphics-driver` | `rendering-domain`, `rendering-wgpu` |
| `physics-core`, `physics-compute` | `physics-domain`, `physics-wgpu` |
| `audio-core`, `audio-driver` | `audio-domain`, `audio-cpal` |
| `engine-core` | `runtime-app` |
| `dsl-core`, `dsl-macros` | `markup`, `markup-macros` |
| `ui-core`, `ui-render`, `ui-mel` | `ui-domain`, `ui-scene2d`, `ui-mel` |
| `scene` | `data-substrate` |
| `editor-protocol`, `editor-host` | `editor-domain`, `editor-app` |

Unchanged: `sdk`, `examples`, the already conforming `*-app` crates,
`diagnostics-fs`, `editor-tcp`.

## Consequences

- `release.sh` and the dependency script work on the new names. The old crates
  on crates.io get a final release that re-exports the new crate; that
  publishing step is not part of this change.
- The `dsl_core` alias inside `meridian-sdk` (emitted by `markup-macros` and
  `ui-mel`) keeps its name until the macro expansion is reworked.
- The agent skill directories keep their old names (`meridian-compute-driver-keeper`
  and so on); their contents use the new crate names.
- Size lints are `deny` from this wave on (see the root `Cargo.toml`).
