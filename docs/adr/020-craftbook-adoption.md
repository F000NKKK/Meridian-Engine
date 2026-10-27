# ADR 020: Adopting Craftbook — runtime, composition root, names, layout

## Status

Accepted. Amends [ADR 003](003-no-global-managers.md),
[ADR 004](004-data-oriented-design.md) and
[ADR 008](008-fixed-point-determinism.md) where noted.

## Context

Rust code in this workspace follows Craftbook
(https://github.com/F000NKKK/craftbook). Its adoption page for Meridian
(`adoption/meridian-engine.md` §5) lists where Meridian's own rules disagree
with the standard. D1 (driver direction) is decided in
[ADR 019](019-ports-in-domain-drivers-as-adapters.md). This ADR records the
others. Each follows the adoption page's recommendation; the wave named is
the migration step that carries it out.

## Decisions

**D2 — `engine-core` becomes a use-case crate, not a hub (wave 6).**
`engine-core` is the runtime context's application layer. It keeps
cross-subsystem orchestration as one use case per frame step; it stops
owning World/Physics/Audio state (`RuntimeState`'s mutexes). State ownership
and wiring move to the composition root. Rule 7 of
[dependency-rules.md](../dependency-rules.md) is read in that sense: "the
place for cross-context coordination", not "the crate that holds
everything". Target name `meridian-runtime-app`.

**D3 — `sdk` is the composition root (wave 6).** `sdk` stays the interface
layer that names concrete adapters and re-exports the public surface. Its
own logic (`scene`, `camera`, `render`, `dsl` tags) moves to the context
crates it belongs to (`FlyCamera` to rendering, DSL tags to a scene
application crate).

**D4 — crate names follow `<product>-<context>-<layer>` (wave 7).**
`meridian-physics-core` becomes `meridian-physics-domain`, drivers become
`-wgpu`/`-cpal` adapters, and so on, per the adoption page's layer map. The
rename is done last, in one change per context, with the old names
deprecated on crates.io by a final release that re-exports the new crate.
Until then the current names stay, and `craftbook.toml` maps them to their
intended layer.

**D5 — float/fixed layout is one directory per concept (wave 4).** The
"three files per crate" rule (`lib.rs` holding generic logic beside
`float.rs`/`fixed.rs`) is replaced by Craftbook §1.3 #4: each concept that
has flavours is a directory, `rotor/mod.rs` (wiring only) with
`rotor/float.rs` and `rotor/fixed.rs`; generic logic shared by both flavours
lives in item-named files (`rotor/rotor.rs` is not allowed — name the file by
its item, e.g. `rotor/rotation.rs`). `generic.rs` is not a file name. The
second half of the old rule stays unchanged: write the algorithm once,
generic over `GaFlavor`; flavour files are type aliases and marker impls.

**D6 — disclosed `f32` duplication stays, documented on each item.**
`Motor3`/`Vec3`/`Bivector3`/`Rotor` remain concrete `f32` because
`gac-compute` dispatches them to the GPU (ADR 008). That is the external
constraint Craftbook §2.5 allows; each such item states it in its own `///`
doc. The files are still split per D5.

**D7 — logging becomes a port (wave 6).** `foundation`'s process-wide log
sink is the one allowed global (Craftbook §4.7, listed in `craftbook.toml`
`[[exempt]]`). Its file writer (tokio) and crash reporter are I/O and move
to an adapter (`meridian-diagnostics-fs`); the kernel keeps `EngineError`
and a `Log` port, and the composition root installs the sink. The same
applies to `asset-core`'s global codec registry: it becomes a value passed in.
This amends ADR 003's reading of the log sink as "not a manager": it is
still not a manager, but it is global state and is exempted by name.

**D8 — performance kernels are declared, and must earn it (wave 3).**
`gac-core`, `ecs-core`, `memory-core` and `physics-core` keep their
data-oriented shape (ADR 004) under Craftbook §7.8: `profile = "perf"` in
`craftbook.toml` and a sentence in the crate's `//!`. The profile is
provisional until each crate has a `criterion` benchmark that justifies it;
wave 3 adds those.

**D9 — dependency docs match the graph (wave 0).** Rule 12 of
dependency-rules.md named only `graphics-driver`/`compute-driver` as users of
`gpu-driver`, while `gac-compute` and `physics-compute` use it too; the rule
now lists all four. `physics-core`'s unused `physics-driver` edge is
removed.

**D10 — one source for layers (wave 0, finished in wave 7).**
`craftbook.toml` declares each crate's layer and context;
`scripts/check_dependency_rules.py` reads it to reject outward edges, and
keeps its exact edge list only to catch undocumented edges. When the pending
list of ADR 019 is empty and the crates are renamed, `craftcheck deps`
replaces the script.

**D11 — agent guides keep only project rules.** `AGENTS.md`, `CLAUDE.md`,
`.claude/skills/*` and `.codex/skills/*` link Craftbook and describe the
target direction of ADR 019; they do not restate the general rules.

## Consequences

- Waves 4–7 of the adoption page can start without further decisions.
- Until wave 7, crate names and layers disagree; `craftbook.toml` is the
  bridge and `docs/context-map.md` the reading aid.
- ADR 003's "no global managers" is unchanged; the log sink is now an
  explicit, single exemption rather than an interpretation.
