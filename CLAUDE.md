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

When starting work, announce the applicable skill set(s) to yourself and follow
the listed workflow.

### Documentation skill: `docs-guardian`

Files: `README.md`, `AGENTS.md`, `CLAUDE.md`, `docs/**`, architectural comments,
crate descriptions.

Use it to keep documentation accurate, concise and aligned with current code.
Update docs in the same change that invalidates them. Prefer ADRs for permanent
architecture decisions and `roadmap.md` for implementation status.

### Dependency skill: `crate-boundary-keeper`

Files: all `Cargo.toml`, new crates, public APIs that move responsibility across
crates.

Use it to reject forbidden edges, design adapter crates when needed, and run
`./build.sh check-deps`. Never add a dependency merely to silence a compiler
error without checking the graph.

### GAC skill: `gac-math-keeper`

Files: `meridian-gac-core`, transform code, spatial math, rotation integration,
physics angular quantities, camera/listener spatial code.

Use it to preserve GA semantics. Angular quantities that live in the rotation
Lie algebra should be bivectors. Validate math against external oracles where
possible.

### ECS skill: `data-oriented-ecs-keeper`

Files: `meridian-ecs-core`, component storage, archetypes, queries, world/entity
logic.

Use it to preserve SoA/archetype storage, avoid domain leakage, and test entity
migration, dead entities, query behavior and type-erased storage boundaries.

### Memory/resource/asset skill: `resource-lifetime-keeper`

Files: `meridian-memory-core`, `meridian-resource-core`, `meridian-asset-core`,
handles, pools, decoders, dependency graph.

Use it to keep generic handles, typed resource IDs, decoded CPU data and
lifetime policy separate. Do not create managers. Test stale handles,
generation changes, cycle checks and malformed asset bytes.

### Compute skill: `compute-driver-keeper`

Files: `meridian-compute-driver`, `meridian-compute-runtime`,
`meridian-gac-compute`, future concrete `*-compute` adapter crates.

Use it to keep runtime dispatch generic, domain kernels in adapters, and
scheduler responsibilities aligned with `task-core`. Test sequential/small and
parallel/large dispatch paths.

### Subsystem skill: `subsystem-core-keeper`

Files: `meridian-graphics-core`, `meridian-physics-core`, `meridian-audio-core`,
corresponding drivers and design docs.

Use it to keep domain algorithms in core crates and hardware/OS execution in
driver crates. Each core may use its own driver, not another subsystem's driver.
Compute work should go through `compute-runtime` or an adapter crate.

### Engine/examples skill: `engine-integration-keeper`

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
