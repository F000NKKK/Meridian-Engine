# Segment: scripting (`team/scripting`)

Ownership note and backlog for the scripting segment: `.mel` (Meridian
Engine Language), its planned scripting extension, a future own language,
and multi-language game logic. This file tracks *who owns what and what is
next*; permanent decisions go into ADRs, implementation status into
[roadmap.md](../roadmap.md).

## What exists today

`.mel` is currently only the declarative scene DSL from
[ADR 015](../adr/015-extensible-scene-dsl.md). There is no script
execution anywhere in the workspace yet.

| Piece | Where | Role |
|---|---|---|
| Markup parser | `crates/meridian-dsl-core` (`parse`, `Element`, `ParseError`) | Domain-blind `<Tag attr="v">` tree; no text nodes, byte-offset errors only |
| Typed tags | `crates/meridian-dsl-core/src/tag.rs` (`DslTag`, `DslRegistry`, `BuiltNode`) | Name -> builder registry, fails closed on unknown tags |
| Tag macro | `crates/meridian-dsl-macros` (`#[dsl_tag(name = "...")]`) | Implements `DslTag` for a plain struct; used only via `meridian_sdk::dsl_tag` |
| Built-ins | `crates/meridian-sdk/src/dsl.rs` (`Scene`/`Entity`/`Mesh`/`Material`/`RigidBody`/`Transform`, `default_registry`, `build_scene`) | Ordinary `#[dsl_tag]` structs, no privilege over game tags |
| Samples | `examples/assets/scenes/{physic_figures,magic_figures}.mel` | `magic_figures` shows game-defined tags (`Glow`/`Orbit`/`Audio`) |

Per-frame behavior (orbit/spin, fixed-step accumulator) is still plain Rust
in the examples; the DSL describes composition only.

## Ownership and boundaries

The segment owns `meridian-dsl-core`, `meridian-dsl-macros`,
`meridian_sdk::dsl`, the `.mel` samples, and every future scripting crate.
It follows the workspace rules in [dependency-rules.md](../dependency-rules.md):

- The language core (lexer, parser, AST, interpreter/VM) stays
  domain-blind, like `dsl-core`: it never names ECS, physics or graphics
  types. Engine bindings live in an adapter crate above it.
- No global script manager or singleton VM ([ADR 003](../adr/003-no-global-managers.md));
  a script runtime is a value owned by whoever composes it (`engine-core`
  or the application).
- Script logic touching fixed-step simulation must be deterministic
  ([ADR 008](../adr/008-fixed-point-determinism.md)): the language needs a
  `Fixed` numeric path, not only `f32`.
- Parsing and running scripts is synchronous; only file watching for hot
  reload is I/O and may be `async` ([ADR 009](../adr/009-async-io-via-tokio.md)).
- Scene/save data formats belong to `team/data` (Substrate-Data);
  editor tooling (LSP/DAP, Meridian IDLE) belongs to `team/ide`. This
  segment provides the language, its diagnostics and a host API, not the
  editor or the storage format.

## Backlog

Ordered roughly by dependency. Nothing here is started.

1. **ADR 016: `.mel` scripting model.** Decide before writing code:
   embedding syntax (Razor-style `@expr` in attributes and `@{ ... }`
   blocks, or a dedicated `<Script>` element), the first execution
   backend (own interpreter vs. embedding an existing VM such as WASM or
   Lua behind the same host API), the value model, and the determinism
   story. Needs the user's call on "own language now vs. later".
2. **Source spans in `dsl-core`.** Add line/column spans to `Element` and
   attributes and to `ParseError`. Prerequisite for script diagnostics and
   for `team/ide`'s LSP; small, non-breaking if added as new fields with
   a constructor change noted in the changelog.
3. **Raw content in the markup grammar.** `dsl-core` rejects text between
   elements; scripts need a raw-body form (`@{ }` or a raw-text element)
   that the parser carries verbatim without interpreting it.
4. **`meridian-script-core` crate.** Domain-blind lexer, parser, AST and a
   tree-walking interpreter as the first backend; fuzz/malformed-input
   tests; no panics on bad source. Tier placement next to `dsl-core`.
5. **Host API and engine bindings.** A trait-based surface (call native
   functions, read/write ECS components through `ecs-core` queries,
   schedule work) in an adapter crate, composed by `engine-core`, so
   additional languages plug into the same bindings (multi-language game
   logic).
6. **Script stage in the frame pipeline.** Run scripts as a stage in
   `meridian_sdk::pipeline` / `engine-core::Runtime`, with a fixed-step
   variant for deterministic logic.
7. **Hot reload.** Watch `.mel` files, re-parse and swap scripts without
   a Rust rebuild; state migration rules for live entities.
8. **Tooling hooks (with `team/ide`).** Expose a diagnostics/symbols API
   that a `.mel` language server can use on top of Substrate's `lsp`
   feature; DAP comes after the VM has stepping support.
9. **Data interop (with `team/data`).** Map script values to
   Substrate-Data `Value`/`Shape` so scripts can read and write scene and
   save data without a second serialization path.

## Open questions for the user

- Own language from the start, or an embedded VM first with the own
  language as a later backend?
- Is Razor-style `@` embedding the agreed syntax, or still open?
- Which languages besides `.mel` must be supported for game logic
  (Rust-only plugins, WASM, Lua, C#)?
