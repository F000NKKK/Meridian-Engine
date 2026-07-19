# Meridian-Engine agent guide

This file applies to the whole repository. Every agent that changes files in
this tree must read this file first, then read the document set relevant to the
files being changed.

## Project idea

Meridian-Engine is a modular Rust engine core for real-time graphics, audio,
physics, simulation and tooling. The central idea is that all spatial systems
share one geometric-algebra model (`Motor3`, rotors, bivectors) instead of each
subsystem inventing separate matrix/quaternion/transform conventions.

The second core idea is strict layering. Low-level crates provide primitives;
domain crates build upward; the application layer owns policy. The project
prefers stable boundaries over quick coupling.

## Source of truth order

When instructions conflict, use this order:

1. Direct user/developer/system instructions.
2. This `AGENTS.md` and `CLAUDE.md`.
3. `docs/dependency-rules.md` for dependency direction and crate boundaries.
4. ADRs in `docs/adr/` for why a decision exists.
5. Subsystem design docs in `docs/*-design.md`, `docs/memory-model.md`, and
   `docs/threading-model.md`.
6. `docs/roadmap.md` for current implementation state and sequencing.
7. Existing code and tests.

If a document is stale, do not silently follow stale text. Update the document
or explicitly explain why the code change intentionally supersedes it.

## Mandatory document workflow

Before changing code, identify which documents govern it:

- Dependency edges, crate layering, or new crates: read
  `docs/dependency-rules.md` and the relevant ADRs.
- Spatial math, transforms, rotation, angular velocity, or coordinate systems:
  read `docs/gac-design.md` and ADR 001.
- Handles, pools, resource identity, dependency graphs, asset lifetime:
  read `docs/memory-model.md`, ADR 002, ADR 003 and ADR 006.
- ECS world/archetypes/components/queries: read `docs/ecs-design.md` and ADR 004.
- Drivers, GPU, audio device, physics backend, platform/hardware boundaries:
  read ADR 005 and the relevant subsystem design doc.
- Compute dispatch, CPU-SIMD/GPU kernels, batched transforms: read ADR 007,
  `docs/threading-model.md`, and `docs/dependency-rules.md` rules 5, 10 and 11.
- Any `async fn`, blocking I/O, or a new `tokio` dependency: read ADR 009.
- Roadmap, implementation order, or deciding what should be built next: read
  `docs/roadmap.md`.

When a change makes a document false, update the document in the same change.
Documentation is not optional bookkeeping; it is part of the architecture.

## Hard architectural rules

Treat `docs/dependency-rules.md` as enforceable. In particular:

- Keep the workspace graph a DAG.
- Do not make a `*-driver` depend on its matching `*-core`.
- Do not make a `*-core` depend on another subsystem's `*-driver`.
- Keep `meridian-gac-core` pure math; no GPU knowledge or compute-runtime edge.
- Keep `meridian-compute-runtime` generic; no `Motor3`, particles, rigid bodies
  or other domain-shaped algorithms/buffers in it.
- Put batched GAC compute in `meridian-gac-compute`, not in `gac-core` or
  `compute-runtime`.
- Keep `asset-core` limited to loading/decoding CPU-side data; no asset manager,
  cache manager, resource manager, handles, or lifetime policy.
- Keep `resource-core` limited to identity/versioning/dependency topology; no
  resource lifetime policy or manager service.
- Keep cross-subsystem coordination in `engine-core`, not in ad hoc dependencies
  between domain crates.

Run `./build.sh check-deps` after any dependency or crate-boundary change.

## Skill routing for agents

Use these repository-local skill sets as working modes. Pick the smallest set
that covers the touched files, and state which set you used in your final note.
Each set below has real skill files under `.codex/skills/<skill-name>/SKILL.md`
and `.claude/skills/<skill-name>/SKILL.md`. Codex-style agents should read the
`.codex` copy; Claude-style agents should read the `.claude` copy. When a set
applies, read the relevant skill file before editing the matching files; the
summary below is only a routing table.

### `meridian-docs-guardian`

Use for: `README.md`, `AGENTS.md`, `CLAUDE.md`, `docs/**`, comments that encode
architecture, crate descriptions in `Cargo.toml`.

Workflow:

1. Read `docs/dependency-rules.md` first for architecture-impacting text.
2. Check whether `docs/roadmap.md` and `docs/architecture.md` agree with the
   current code before repeating status claims.
3. Prefer precise, falsifiable statements over aspirational text.
4. If changing a decision, add or update an ADR instead of burying rationale in
   a code comment.
5. Validate Markdown links where practical and run at least `./build.sh
   check-deps` when dependency rules are mentioned.

### `meridian-crate-boundary-keeper`

Use for: any `Cargo.toml`, new crate, moved type, public API that changes what
one crate knows about another crate.

Workflow:

1. Inspect the dependency edge before adding it.
2. Confirm the edge is permitted by `docs/dependency-rules.md`.
3. Prefer adding a small adapter crate over reversing a dependency direction.
4. Never solve a compile error by adding a dependency that violates layering.
5. Run `./build.sh check-deps` and `cargo test --workspace`.

### `meridian-gac-math-keeper`

Use for: `crates/meridian-gac-core/**`, transform code, physics rotation,
spatial audio positioning, camera transforms, and any `Motor3`/`Rotor`/
`Bivector3` use.

Workflow:

1. Read `docs/gac-design.md` and ADR 001.
2. Preserve the distinction between vectors and bivectors; angular quantities
   are bivectors when physics/rotation semantics require it.
3. Add tests against an independent oracle where possible, not just
   self-consistency between project functions.
4. Keep CPU feature detection/SIMD dispatch outside `gac-core`; use
   `numeric-core` or compute adapter crates instead.

### `meridian-data-oriented-ecs-keeper`

Use for: `crates/meridian-ecs-core/**` and components/query/storage changes.

Workflow:

1. Read `docs/ecs-design.md` and ADR 004.
2. Preserve archetype/SoA storage assumptions.
3. Do not make ECS understand graphics, physics, audio, asset or gameplay
   domain semantics. `Transform` is the narrow shared spatial primitive.
4. Be especially careful with aliasing and type erasure; add tests for entity
   movement between archetypes and stale/dead entity behavior.

### `meridian-resource-lifetime-keeper`

Use for: `crates/meridian-memory-core/**`, `crates/meridian-resource-core/**`,
`crates/meridian-asset-core/**`, handle types, pools, decoders and dependency
tracking.

Workflow:

1. Read `docs/memory-model.md`, ADR 002, ADR 003 and ADR 006.
2. Keep memory handles generic in `memory-core` and resource identities typed in
   `resource-core`.
3. Keep decoded CPU data in `asset-core`; mint handles outside `asset-core`.
4. Do not add managers or singletons.
5. Test stale handles, generation bumps, dependency cycles and malformed asset
   inputs.

### `meridian-compute-driver-keeper`

Use for: `crates/meridian-compute-driver/**`,
`crates/meridian-compute-runtime/**`, `crates/meridian-gac-compute/**`, GPU/SIMD
dispatch, buffers, kernels and scheduling integration.

Workflow:

1. Read ADR 007 and `docs/threading-model.md`.
2. Keep `compute-runtime` about dispatch mechanics only.
3. Put domain kernels in adapter crates (`gac-compute` now; future
   `*-compute` crates only when concrete need exists).
4. Do not duplicate general task scheduling inside compute runtime; use
   `task-core` boundaries intentionally.
5. Test both small batches and batches large enough to exercise parallel paths.

### `meridian-subsystem-core-keeper`

Use for: `graphics-core`, `physics-core`, `audio-core`, and their design docs.

Workflow:

1. Read the matching `docs/*-design.md` plus dependency rules.
2. Keep domain algorithms in `*-core`, not in `*-driver`.
3. Use only the matching driver crate, never another subsystem's driver.
4. Use `compute-runtime`/adapter crates for compute work.
5. Add tests around domain invariants: collision pairs, render graph ordering,
   mixer spatial behavior, etc.

### `meridian-engine-integration-keeper`

Use for: `crates/meridian-engine-core/**`, examples, frame scheduling,
cross-subsystem orchestration.

Workflow:

1. Read `docs/architecture.md`, `docs/threading-model.md`, and
   `docs/roadmap.md`.
2. Put cross-subsystem coordination here rather than creating domain crate
   cross-dependencies.
3. Keep examples small and demonstrative; examples may compose crates, but core
   crates must remain layered.
4. Run relevant examples with `./build.sh run <example>` when changed.

## Commit discipline

Keep changes coherent. If code and docs both need updates, commit them
together. Use commit messages that name the affected boundary or subsystem
(e.g. `gac-core`, `ecs-core`, `docs`). Do not leave workspace status dirty
unless explicitly asked.

Do not add the AI agent as a co-author of a commit. Commit messages must not
include a `Co-Authored-By` (or equivalent) line attributing the commit to
Claude, Codex, or any other AI agent.

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
actually has a GPU-dispatch (or similarly hard) constraint forcing it; if
not, genericize instead.

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
`docs/dependency-rules.md` and the crate's own top-of-file doc comment
before writing new code, not after.

## Async only on genuine I/O

This is an async-native engine: `tokio` is the workspace's async runtime
(see ADR 009). A function becomes `async fn` only when it performs
genuine I/O — completion time that is unbounded and determined by
something outside this process (an OS/driver handshake, a GPU/device
queue, a filesystem, a network peer): the textbook definition, not
"talks to an external system's *type*." `meridian-graphics-driver::Device::new`/
`read_buffer` are the concrete precedent. Recording/allocation/validation
calls with bounded, effectively-instant local cost stay plain synchronous
functions — `create_buffer`, `write_buffer`, `CommandBuffer::submit`, and
their future counterparts elsewhere (decoders taking already-in-memory
bytes, ECS queries, GA math) never become `async` just for API
consistency; that adds `Future`-polling/executor overhead to hot-path
code with nothing to gain from it. Only add `tokio` as a dependency to a
crate that has a genuine I/O operation of its own — it is not a default
every crate picks up. When a blocking call is unavoidable inside an
`async fn` (e.g. manually pumping `wgpu::Device::poll` because `wgpu` has
no reactor integration), run it via `tokio::task::spawn_blocking`, not
inline on the async task.

## Coding and testing expectations

- Use Rust 2024 and the workspace Rust version from `Cargo.toml`.
- Prefer standard library implementations until a document explicitly accepts an
  external dependency. The GPU backend is documented as a deliberate future
  exception via `wgpu`.
- Do not add speculative subsystems or crates. Follow `docs/roadmap.md`.
- Add focused tests with each behavioral change.
- For general verification, run:
  - `./build.sh check-deps`
  - `cargo test --workspace`
- For examples, use `./build.sh run <example> -- <args>`.

## Final response expectations for agents

When reporting work, include:

- What changed and why.
- Which skill routing set(s) were used.
- Which documents governed the change.
- Exact tests/checks run and their result.
- Any known stale documents or follow-up work.
