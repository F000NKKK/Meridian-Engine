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
- Keep `gac-core` independent from `compute-runtime`.
- Keep `compute-runtime` independent from `gac-core` and free of domain-shaped
  algorithms or buffers.
- Keep driver crates below their domain cores; drivers do not know domain
  objects.
- Keep asset loading separate from resource identity and lifetime.
- Keep resource identity separate from resource lifetime policy.
- Keep ECS generic and data-oriented; do not leak graphics/physics/audio domain
  concepts into it.
- Put cross-subsystem orchestration in `engine-core`.
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

Use it to reject forbidden edges, design adapter crates when needed, and run
`./build.sh check-deps`. Never add a dependency merely to silence a compiler
error without checking the graph.

### GAC skill: `meridian-gac-math-keeper`

Files: `meridian-gac-core`, transform code, spatial math, rotation integration,
physics angular quantities, camera/listener spatial code.

Use it to preserve GA semantics. Angular quantities that live in the rotation
Lie algebra should be bivectors. Validate math against external oracles where
possible.

### ECS skill: `meridian-data-oriented-ecs-keeper`

Files: `meridian-ecs-core`, component storage, archetypes, queries, world/entity
logic.

Use it to preserve SoA/archetype storage, avoid domain leakage, and test entity
migration, dead entities, query behavior and type-erased storage boundaries.

### Memory/resource/asset skill: `meridian-resource-lifetime-keeper`

Files: `meridian-memory-core`, `meridian-resource-core`, `meridian-asset-core`,
handles, pools, decoders, dependency graph.

Use it to keep generic handles, typed resource IDs, decoded CPU data and
lifetime policy separate. Do not create managers. Test stale handles,
generation changes, cycle checks and malformed asset bytes.

### Compute skill: `meridian-compute-driver-keeper`

Files: `meridian-compute-driver`, `meridian-compute-runtime`,
`meridian-gac-compute`, future concrete `*-compute` adapter crates.

Use it to keep runtime dispatch generic, domain kernels in adapters, and
scheduler responsibilities aligned with `task-core`. Test sequential/small and
parallel/large dispatch paths.

### Subsystem skill: `meridian-subsystem-core-keeper`

Files: `meridian-graphics-core`, `meridian-physics-core`, `meridian-audio-core`,
corresponding drivers and design docs.

Use it to keep domain algorithms in core crates and hardware/OS execution in
driver crates. Each core may use its own driver, not another subsystem's driver.
Compute work should go through `compute-runtime` or an adapter crate.

### Engine/examples skill: `meridian-engine-integration-keeper`

Files: `meridian-engine-core`, `examples/**`, frame scheduler and subsystem
composition.

Use it to put orchestration at the top layer, keep examples demonstrative, and
avoid changing lower crates just to satisfy an example convenience.

## Validation checklist

Run the narrowest useful command set, and prefer more checks when touching
architecture:

```sh
./build.sh check-deps
cargo test --workspace
```

Run package-specific tests during iteration, for example:

```sh
cargo test -p meridian-gac-core
cargo test -p meridian-ecs-core
cargo test -p meridian-resource-core
```

Run changed examples through `./build.sh run <example>`.

## Commit discipline

Keep changes coherent. If code and docs both need updates, commit them together.
Use commit messages that name the affected boundary or subsystem. Do not leave
workspace status dirty unless explicitly asked.

## Float/Fixed branching: always 3 files

Whenever a crate's code branches for a float (`f32`/`Scalar`) flavor and a
`Fixed` (deterministic) flavor, split it into exactly three files:
`lib.rs` (shared/generic logic, or module wiring + re-exports), `float.rs`
(or a domain-suffixed name like `float_ga.rs` for an existing convention),
`fixed.rs`/`fixed_ga.rs`. Never leave float-flavored code loose in `lib.rs`
next to a separate `fixed`/`deterministic.rs` module — the split must be
symmetric on both sides. See `meridian-numeric-core` (`lib.rs`/`float.rs`/
`fixed.rs`) and `meridian-gac-core` (`lib.rs`/`float_ga.rs`/`fixed_ga.rs`)
for the pattern.

**Default to generic (or thin wrappers over one generic implementation),
not disclosed duplication.** Full duplication (writing the float and fixed
versions of a type/algorithm twice, even if "disclosed" with matching doc
comments) is justified *only* when a genuine external constraint makes
genericity impossible or unsafe — the one known case in this workspace is
`gac-core::Motor3`/`Vec3`/`Bivector3`/`Rotor`: `meridian-gac-compute`
dispatches them to GPU, GPUs have no real `i64` support and are
`f32`-native, so a GPU-dispatchable type has to stay concretely `f32`
regardless of how the CPU-side type is parameterized (see
[ADR 008](docs/adr/008-fixed-point-determinism.md)). Absent a constraint
like that, write the algorithm/type once, generic over a trait describing
what it needs from "a GA flavor" (see `meridian_gac_core::GaFlavor` and
its component traits), and make `float.rs`/`fixed.rs` thin — type aliases
and flavor-marker impls, not a second copy of the logic. Before
duplicating float/fixed code, check whether the thing being duplicated
actually has a GPU-dispatch (or similarly hard) constraint forcing it;
if not, genericize instead. `physics-core`'s `Integrator`/
`ConstraintSolver`/`BroadPhase`/`NarrowPhase`/`RigidBody` and
`gac-core`'s `Aabb`/`Sphere`/`Obb`/`Cone`/`Plane`/`ConvexVolume`/`Shape`
are generic over `GaFlavor` for exactly this reason — none of them are
GPU-dispatched.

## Do not drag another crate's logic into your own

If a piece of logic conceptually belongs to another crate (geometry
belongs to `gac-core`, not to whichever crate needed it first; dispatch
mechanics belong to `compute-runtime`, not to a domain crate), depend on
that crate and reuse its implementation — do not hand-roll a local copy
inside the crate that merely *consumes* it. This applies even under time
pressure or when the "local copy" looks small: `physics-core` reinventing
sphere-overlap math or bounding-box structs instead of using
`gac-core::Aabb`/`Shape` is the concrete case this rule was written after
catching. When in doubt about which crate owns a piece of logic, check
[dependency-rules.md](docs/dependency-rules.md) and the crate's own
top-of-file doc comment before writing new code, not after.
