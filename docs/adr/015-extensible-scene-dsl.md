# ADR 015: Extensible scene DSL, not a fixed schema

## Status

Accepted.

## Context

The SDK's examples compose scenes (entities, meshes, materials, rigid
bodies) entirely in Rust `main()`/`on_ready` code — real, but verbose
boilerplate an application author repeats for every scene. A first
proposal introduced a fixed XAML-like schema hardcoded into the SDK
(`<App><Scene><Entity><Mesh><Material><RigidBody>`, one Rust type per
tag, enumerated centrally). That was rejected: a fixed schema means
every new kind of thing a game wants to describe declaratively (a
custom component, a game-specific system hookup) requires editing
`meridian-sdk` itself — exactly the kind of hub-crate coupling
`docs/dependency-rules.md` rule 7 exists to prevent, just moved from
Cargo dependencies into a closed tag vocabulary instead.

## Decision

Split the DSL into two crates plus one SDK module, so no crate needs to
enumerate tags centrally:

- **`meridian-dsl-core`** — a domain-blind parser: `<Tag attr="value">
  ...children... </Tag>` text into a generic `Element` tree,
  plus `DslTag`/`DslRegistry` (a "typed thing buildable from
  attributes" trait and a name -> builder map). Never changes when a
  new tag is added, because it never encodes tag names or their
  meaning at all.
- **`meridian-dsl-macros`** — `#[dsl_tag(name = "...")]`, an attribute
  macro that implements `DslTag` for any plain struct with named
  fields (one required attribute per field, `Option<T>` fields
  optional). This is the whole extension mechanism: a game developer
  annotates their own struct, in their own crate, and it participates
  in the same DSL as every built-in tag — no `meridian-sdk` PR
  required.
- **`meridian_sdk::dsl`** — a handful of built-in tags
  (`Entity`/`Mesh`/`Material`/`RigidBody`/`Transform`) defined with the
  exact same `#[dsl_tag]` macro a game would use for its own, plus
  `default_registry()` (built-ins pre-registered) and `build_scene()`
  (parse + build in one call). Nothing here is privileged over a
  custom tag; `default_registry()` is a convenience starting point, not
  the only valid one.

Scope, deliberately: the DSL describes scene/entity composition only —
no window title, logging config, or crash-report directory (an
"app-shell" schema was considered and deferred; see the roadmap for
that as a possible later phase). An application still writes that part
in plain Rust `main()`.

## Consequences

- Adding a new tag is: write a struct, annotate it, call
  `registry.register::<T>()`. No `meridian-sdk` release needed, no
  central enum/match to extend.
- `meridian-dsl-macros`' generated code names `meridian_sdk::dsl_core`
  (resolved via `proc-macro-crate` to `crate` when expanding inside
  `meridian-sdk` itself, `::meridian_sdk` otherwise) — the macro is
  only meant to be used through `meridian_sdk::dsl_tag`, consistent
  with this workspace's rule that applications reach everything
  through `meridian-sdk` alone; it isn't a general-purpose
  "attribute struct" macro usable without the SDK.
- `DslRegistry::build` fails closed on an unregistered tag name (a
  typo surfaces as a `TagParseError`, never a silently-skipped node),
  and both `meridian-dsl-core`'s error types implement
  `meridian_foundation::EngineError`, so a caller that lets a DSL error
  panic gets it captured in the crash report like any other engine
  error — no dead ends.
- Downcasting a `BuiltNode`'s type-erased value by tag name (rather
  than a typed enum) means a caller must know which Rust type a given
  tag name maps to — true for built-ins (documented in
  `meridian_sdk::dsl`) and for a game's own tags (it wrote them). This
  is the same tradeoff `ecs-core`'s type-erased component storage
  makes, for the same reason: the registry can't have compile-time
  knowledge of types it doesn't know exist yet.
