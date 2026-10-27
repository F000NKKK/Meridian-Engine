# CLAUDE.md — Meridian-Engine operating guide

Read this file before working in the repository. It mirrors `AGENTS.md` for
Claude-style agents and is scoped to the whole tree.

## Mission

Meridian-Engine is a modular Rust engine core built around shared geometric
algebra and strict subsystem boundaries. The project is not trying to become a
large monolithic engine. It is building a small, stable set of crates where
math, memory, resources, ECS, compute, drivers and domain cores each have one
clear responsibility.

The design goal is: one spatial model, no global managers, no accidental hub
crates, no hidden lifetime policy in loader/identity crates, and no driver/core
coupling.

## Engineering standard

Rust code here follows **Craftbook**, our engineering standard:
https://github.com/F000NKKK/craftbook. Read its `README.md` and the relevant
`rust/` chapter before writing or reviewing code; the short version is in
[`.claude/rules/craftbook.md`](.claude/rules/craftbook.md). The conflicts
between Craftbook and this repository's former rules are decided in
[ADR 019](docs/adr/019-ports-in-domain-drivers-as-adapters.md) (driver
direction) and [ADR 020](docs/adr/020-craftbook-adoption.md) (runtime hub,
composition root, crate names, float/fixed layout, log sink, performance
kernels); Craftbook's `adoption/meridian-engine.md` holds the migration plan.
Code not yet migrated is tracked by baselines (`.craftcheck-baseline`,
`.craftcheck-deps-baseline`, `.clippy-backlog`, `PENDING` in
`scripts/check_dependency_rules.py`); a change may shrink them, never grow
them.

## Read order

For any task, first read `AGENTS.md`. Then read the narrowest applicable docs:

- `docs/dependency-rules.md` for crate graph and forbidden edges.
- `docs/architecture.md` for high-level layering.
- `docs/roadmap.md` for current state and next implementation order.
- `docs/adr/*.md` for accepted decisions and rationale.
- `docs/gac-design.md`, `docs/ecs-design.md`, `docs/graphics-design.md`,
  `docs/physics-design.md`, `docs/memory-model.md`, `docs/threading-model.md`
  for subsystem-specific work.

If docs and code disagree, stop and resolve the mismatch as part of the work or
call it out explicitly. Do not preserve stale claims just because they are in a
Markdown file.

## Non-negotiable rules

- Keep dependency direction consistent with `docs/dependency-rules.md`.
- Keep `gac` independent from `compute-app`.
- Keep `compute-app` independent from `gac` and free of domain-shaped
  algorithms or buffers.
- Dependencies point inwards (ADR 019): domain and application crates own
  the port traits they need; driver crates are adapters that implement them
  and know only the port's types. A domain or application crate never takes
  a new edge to a driver; the edges pending migration only shrink.
- Keep asset loading separate from resource identity and lifetime.
- Keep resource identity separate from resource lifetime policy.
- Keep ECS generic and data-oriented; do not leak graphics/physics/audio domain
  concepts into it.
- Put cross-subsystem orchestration in `runtime-app`.
- Do not introduce global singleton managers.

## Skill sets to invoke by file area

When starting work, read the applicable `.claude/skills/<skill-name>/SKILL.md`
file(s), announce the applicable skill set(s) to yourself, and follow the listed
workflow. `.codex/skills/<skill-name>/SKILL.md` mirrors the same skill set for
Codex-style agents; the sections below are routing summaries, while the skill
files are the operational instructions.

### Documentation skill: `meridian-docs-guardian`

Files: `README.md`, `AGENTS.md`, `CLAUDE.md`, `docs/**`, architectural comments,
crate descriptions.

Use it to keep documentation accurate, concise and aligned with current code.
Update docs in the same change that invalidates them. Prefer ADRs for permanent
architecture decisions and `roadmap.md` for implementation status.

### Dependency skill: `meridian-crate-boundary-keeper`

Files: all `Cargo.toml`, new crates, public APIs that move responsibility across
crates.

Use it to reject forbidden edges (anything pointing outwards per
`craftbook.toml` layers), design ports and adapter crates when needed, and run
`./build.sh check-deps` and `craftcheck deps`. Never add a dependency merely to silence a compiler
error without checking the graph.

### GAC skill: `meridian-gac-math-keeper`

Files: `meridian-gac`, transform code, spatial math, rotation integration,
physics angular quantities, camera/listener spatial code.

Use it to preserve GA semantics. Angular quantities that live in the rotation
Lie algebra should be bivectors. Validate math against external oracles where
possible.

### ECS skill: `meridian-data-oriented-ecs-keeper`

Files: `meridian-ecs`, component storage, archetypes, queries, world/entity
logic.

Use it to preserve SoA/archetype storage, avoid domain leakage, and test entity
migration, dead entities, query behavior and type-erased storage boundaries.

### Memory/resource/asset skill: `meridian-resource-lifetime-keeper`

Files: `meridian-memory`, `meridian-resource`, `meridian-asset-domain`,
handles, pools, decoders, dependency graph.

Use it to keep generic handles, typed resource IDs, decoded CPU data and
lifetime policy separate. Do not create managers. Test stale handles,
generation changes, cycle checks and malformed asset bytes.

### Compute skill: `meridian-compute-driver-keeper`

Files: `meridian-compute-wgpu`, `meridian-compute-app`,
`meridian-gac-wgpu`, future concrete `*-compute` adapter crates.

Use it to keep runtime dispatch generic, domain kernels in adapters, and
scheduler responsibilities aligned with `runtime-threads`. Test sequential/small and
parallel/large dispatch paths.

### Subsystem skill: `meridian-subsystem-core-keeper`

Files: `meridian-rendering-domain`, `meridian-physics-domain`, `meridian-audio-domain`,
corresponding drivers and design docs.

Use it to keep domain algorithms in core crates and hardware/OS execution in
driver crates. A core declares the ports it needs; its driver implements them
(ADR 019). No core takes a new edge to any driver. Compute work goes through
`compute-app` or an adapter crate.

### Engine/examples skill: `meridian-engine-integration-keeper`

Files: `meridian-runtime-app`, `examples/**`, frame scheduler and subsystem
composition.

Use it to put orchestration at the top layer, keep examples demonstrative, and
avoid changing lower crates just to satisfy an example convenience.

## Validation checklist

Run the narrowest useful command set, and prefer more checks when touching
architecture:

```sh
./build.sh check-deps
./build.sh clippy-backlog
cargo test --workspace
python3 <craftbook>/tools/craftcheck check . --baseline .craftcheck-baseline
python3 <craftbook>/tools/craftcheck deps . --baseline .craftcheck-deps-baseline
```

Tests that need a real GPU or audio device are behind the `hw-tests` feature
(`cargo test --workspace --all-features` on a machine that has them).

Run package-specific tests during iteration, for example:

```sh
cargo test -p meridian-gac
cargo test -p meridian-ecs
cargo test -p meridian-resource
```

Run changed examples through `./build.sh run <example>`.

## Commit discipline

Keep changes coherent. If code and docs both need updates, commit them together.
Use commit messages that name the affected boundary or subsystem. Do not leave
workspace status dirty unless explicitly asked.

## Float/Fixed branching: one directory per concept

Decided in [ADR 020](docs/adr/020-craftbook-adoption.md) D5, following
Craftbook §1.3–1.4. When a concept has a float (`f32`/`Scalar`) flavor and a
`Fixed` (deterministic) flavor, it is a module directory of its own:
`rotor/mod.rs` (wiring only: `mod` and `pub use`), `rotor/float.rs`,
`rotor/fixed.rs`, plus item-named files for logic shared by both flavors.
The split is symmetric: never float code loose next to a `fixed` module.
`lib.rs` is wiring only, and `generic.rs` is not a file name. Wave 4 of
the Craftbook migration converted `gac` (`rotor/`, `motor/`, `vector/`,
`volume/`, ...), `physics-domain` (`collision/`, `dynamics/`, `soft_body/`) and
`physics-wgpu` (`soft_body/`, `broad_phase/`) to this layout; follow it.

**Default to generic (or thin wrappers over one generic implementation),
not disclosed duplication.** Full duplication (writing the float and fixed
versions of a type/algorithm twice, even if "disclosed" with matching doc
comments) is justified *only* when a genuine external constraint makes
genericity impossible or unsafe — the one known case in this workspace is
`gac::Motor3`/`Vec3`/`Bivector3`/`Rotor`: `meridian-gac-wgpu`
dispatches them to GPU, GPUs have no real `i64` support and are
`f32`-native, so a GPU-dispatchable type has to stay concretely `f32`
regardless of how the CPU-side type is parameterized (see
[ADR 008](docs/adr/008-fixed-point-determinism.md)). Absent a constraint
like that, write the algorithm/type once, generic over a trait describing
what it needs from "a GA flavor" (see `meridian_gac::GaFlavor` and
its component traits), and make `float.rs`/`fixed.rs` thin — type aliases
and flavor-marker impls, not a second copy of the logic. Before
duplicating float/fixed code, check whether the thing being duplicated
actually has a GPU-dispatch (or similarly hard) constraint forcing it;
if not, genericize instead. `physics-domain`'s `Integrator`/
`ConstraintSolver`/`BroadPhase`/`NarrowPhase`/`RigidBody` and
`gac`'s `Aabb`/`Sphere`/`Obb`/`Cone`/`Plane`/`ConvexVolume`/`Shape`
are generic over `GaFlavor` for exactly this reason — none of them are
GPU-dispatched.

## Do not drag another crate's logic into your own

If a piece of logic conceptually belongs to another crate (geometry
belongs to `gac`, not to whichever crate needed it first; dispatch
mechanics belong to `compute-app`, not to a domain crate), depend on
that crate and reuse its implementation — do not hand-roll a local copy
inside the crate that merely *consumes* it. This applies even under time
pressure or when the "local copy" looks small: `physics-domain` reinventing
sphere-overlap math or bounding-box structs instead of using
`gac::Aabb`/`Shape` is the concrete case this rule was written after
catching. When in doubt about which crate owns a piece of logic, check
[dependency-rules.md](docs/dependency-rules.md) and the crate's own
top-of-file doc comment before writing new code, not after.

## Async only on genuine I/O

Reading asset files is the exception: it blocks a background thread
(`BackgroundPool`, [ADR 027](docs/adr/027-background-asset-loading.md)) and is
not `async`. This is an async-native engine: `tokio` is the workspace's async runtime
(see [ADR 009](docs/adr/009-async-io-via-tokio.md)). A function becomes
`async fn` only when it performs genuine I/O — completion time that is
unbounded and determined by something outside this process (an OS/driver
handshake, a GPU/device queue, a filesystem, a network peer): the
textbook definition, not "talks to an external system's *type*."
`meridian-rendering-wgpu::Device::new`/`read_buffer` are the concrete
precedent. Recording/allocation/validation calls with bounded,
effectively-instant local cost stay plain synchronous functions —
`create_buffer`, `write_buffer`, `CommandBuffer::submit`, and their
future counterparts elsewhere (decoders taking already-in-memory bytes,
ECS queries, GA math) never become `async` just for API consistency; that
adds `Future`-polling/executor overhead to hot-path code with nothing to
gain from it. Only add `tokio` as a dependency to a crate that has a
genuine I/O operation of its own — it is not a default every crate picks
up. When a blocking call is unavoidable inside an `async fn` (e.g.
manually pumping `wgpu::Device::poll` because `wgpu` has no reactor
integration), run it via `tokio::task::spawn_blocking`, not inline on the
async task.
