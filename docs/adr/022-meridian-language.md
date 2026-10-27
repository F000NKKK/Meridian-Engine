# ADR 022: the Meridian language, a compiled language

## Status

Proposed (2026-10-27, revised the same day). The owner asked for
"something like the language in Yog", then fixed the central point: the
language is **compiled, not a scripting language**. This revision replaces
the first draft, which put a reference interpreter first. Nothing below is
implemented. Order of work (owner): resources, hot reload, then this.

## Context

[ADR 017](017-meui-markup-and-code-behind.md) joins `.meui` markup to
code-behind in Rust, compiled with the game. That covers engine code and
trusted game logic. It does not cover logic written in a language of the
engine's own: gameplay, mods, UI behavior, and code that runs in the
fixed-step deterministic path ([ADR 008](008-fixed-point-determinism.md)).

Yog-Mod-Loader's `.ys` is the precedent, and it is compiled too. From its
README, `docs/design/ys.md` and the `yog-script-*` crates:

- One front end (`yog-script-analyze`: lexer, macros, parser, resolver,
  type and borrow checker) with one diagnostics type shared by the CLI,
  the build and the editor.
- A compiler backend (`yog-script-java`) that emits Java source, which
  `javac` builds; constructs it cannot compile yet fail loudly (`YS0032`)
  instead of miscompiling.
- A small tree-walking interpreter (`yog-script-run`) that is not the
  runtime: it runs build-time macros and `test` blocks and defines the
  semantics the backend must keep.
- A host surface that is the project's own API, not the game's; typed ids
  bound at build time; a panic in a handler disables that handler, not the
  game.
- Hot reload and the debugger exist for native Rust mods, not for the
  language; no WASM, no step budget, no determinism story.

What is game-specific there (sides, versions, patches, JVM) does not carry
over. The full C# surface and borrow checking over a GC are more than this
engine needs.

## Options

1. **Compile to Rust source.** The generator writes `.g.rs` from a
   `build.rs`, as `ui-mel` already does for markup. No runtime, native
   speed, the `Fixed` type is the engine's own, Rust tooling debugs and
   profiles it. Cost: a Rust toolchain and a rebuild per change, and
   errors must be caught by our own checker so they do not surface as
   errors in generated code.
2. **Own backend to WASM (or native through Cranelift).** Fast iteration
   (a module is a file), sandboxing and fuel limits, mods without a
   toolchain. Cost: owning a backend and the host ABI.
3. **Embed an interpreter** (Rhai, Lua, an own VM). Rejected by the
   owner's decision: the language is compiled. An interpreter survives
   only as the small compile-time evaluator described below.

## Decision

A compiled, statically typed language with one front end and several
backends. Option 1 is the first backend and option 2 the second; both
must agree on the semantics, as Yog's backend and interpreter do.

- **Front end (`lang-core`, domain-blind).** Lexer, parser, name and type
  checking, ownership by value / `in` / `out` (no borrow checker over a
  GC), diagnostics with spans and stable codes. It names no engine
  concept. The same diagnostics feed the build, hot reload and the `.meui`
  language server on Substrate's `lsp`.
- **Typed IR between front end and backends.** A backend takes the checked
  IR; a construct a backend lacks is a compile error, never a silent
  fallback.
- **Rust backend first.** It generates `.g.rs` modules like the markup
  glue (`mel_include!`-style), so a `.meui` `{x:Bind}` or event handler can
  name a function of the language the way it names a Rust method today.
  `Fixed`, `Handled` and the UI types map to the engine's own types.
- **One language, two targets (owner, 2026-10-27).** The WASM backend
  stays, and its modules are written in the same `.mel` syntax as the
  Rust backend's: no second dialect. The simpler syntax is what makes the
  stronger isolation practical: a WASM module sees only a capability-scoped
  host API (a capability is granted per module, none by default), runs
  under a fuel limit, and has no filesystem or network unless granted.
- **WASM backend second,** for what must change without a rebuild of the
  program: mods and live iteration. A module is loaded by the host,
  swapped at a frame boundary (the generation and in-flight-guard pattern
  from `yog-hot-reload`), runs under a fuel limit, and sees only the host
  API. State lives in host-owned structs, so a swap keeps it.
- **Compile-time evaluator, not a runtime.** A small interpreter for
  constants, macros and `test` blocks, the role `yog-script-run` plays. It
  is part of the compiler, never shipped with the game.
- **Host API as ports** (ADR 019): traits declared by the domain crates
  (UI, scene, input, time) and adapted by a host crate; typed ids checked
  at compile time, no stringly-typed host calls.
- **Events and commands** follow the WinUI 3 model of
  [ADR 018](018-ui-architecture.md) and `docs/meui-events.md`: handlers are
  named in markup and bound at compile time; a panic disables the handler
  and is reported with `file:line`.
- **Determinism.** A module marked `fixed` may use only `Fixed`, integers
  and host calls declared deterministic; the checker rejects `f32` and
  unordered iteration there.
- **Naming (owner, 2026-10-27).** **MEL is Meridian Engine Language**
  (owner-confirmed). It is a compiled language, not a "script language".
  Its files use **`.mel`**; UI markup moves to
  **`.meui`**; resources stay `.mres`. The markup and its generator keep
  their current crate and identifier names (`ui-mel`, `MelDocument`,
  `mel_include!`) until the engine freeze lifts; renaming them to `meui`
  is a separate change.
- **Pairing.** A UI is a `.meui` markup file plus a `.mel` code-behind
  file (ADR 017's code-behind, written in MEL instead of Rust). Both are
  compiled to Rust (`.g.rs`), the engine's native and fast target, so the
  Rust backend is the primary one.
- **Syntax (owner, 2026-10-27): C#-like with Rust-style ownership.** C#
  surface syntax (classes, properties, `in`/`out`), ownership by move and
  borrow as in Rust, no garbage collector. Errors are values:
  `Result<T, E>` with a propagation operator, no exceptions. Enums carry
  data per variant (Rust-style algebraic enums) and are consumed by
  exhaustive `match`; the checker rejects a missing variant.
- **Shape: Yog Script, aimed at the engine.** Same family as Yog's `.ys`
  (the owner's reference), but the host surface is the engine's, not a
  game's. Yog's logic is ported, not copied: it lives in a few very large
  files there and is hard to read and maintain.
- **Source layout of the compiler: one concept per file.** Every
  keyword, every expression form and every statement form is its own
  file in `lang-core`, with its lexing rule, parser, checker rule, IR
  lowering and tests next to it (a directory per concept, as
  `rotor/{float,fixed}.rs` is in `gac-core`). Orchestration (pipeline
  order, pass scheduling, diagnostics collection) is separate, small and
  names no concept, so adding a construct means adding a directory and one
  registration line. No boilerplate tables and no giant `match` over all
  syntax: a concept registers itself through a trait that the pipeline
  iterates. Craftbook limits apply (files at most 400 lines, one primary
  item per file, no `utils`).

## What MEL takes from Yog Script

Read in `yog-mod-loader/docs/design/ys.md` (2026-10-27). Carried over
as the starting point, each to be fixed in the grammar document:

- **Surface:** types first, `let` / `let mut`, `;`, PascalCase types and
  members, camelCase locals, string interpolation `{expr}`, `switch`
  expressions, records (`record Pos(i32 X, i32 Y, i32 Z)`), structs,
  interfaces, generics with `where` constraints, extension members,
  attributes.
- **Ownership, written as C# writes it:** `T x` copy or `move x`,
  `in T` (`&T`), `ref T` (`&mut T`), `out T`, `move T`; non-lexical
  borrow rules; lifetimes elided at signatures and written (`'a`) only
  where ambiguous; `ref struct` holds borrows; drop at scope end.
  **Difference:** Yog keeps a GC beneath as a safety net because its JVM
  backend needs one; MEL's primary target is Rust, where the same rules
  map one to one onto `T`, `&T`, `&mut T` and `Drop`, so MEL has no GC.
  Shared ownership is an explicit `Shared<T>` / `Weak<T>`.
- **Failures:** `Result<T, E>` with `Ok`/`Err`, `try` (Rust's `?`) with
  `From`-style error conversion, `Map`/`MapErr`/`AndThen`/`UnwrapOr`;
  bugs are panics, there are no exceptions; a dropped `Result` warns.
- **Enums with data:** `enum Mode { Idle, Charging(i32 Rate), Full }`,
  case numbers for data-less cases, matched by exhaustive `switch`
  patterns (`Mode.Charging(let r)`, guards with `when`, ranges, lists).
- **Macros:** Rust-style declarative (`macro` with fragments) and
  procedural macros written in MEL and run by the compile-time
  evaluator; derives and attribute macros; expansion errors point at the
  call. This is the evaluator's main job.
- **Components and systems:** a `component` carries state and behaviour,
  a `system` is a query over components. In the engine this maps to
  `ecs-core` archetypes, so a system is a generated query, not a scan.
- **Failure isolation:** a panicking handler is disabled and reported at
  its `.mel` line; the frame goes on.
- **Tooling:** `test` blocks, `tune` live values, diagnostics with stable
  codes shared by CLI, build and editor.

Left behind as game-specific: sides as types, Minecraft content kinds,
`.yns` patches and class indexes, versions and `targets`. Sides may
return as editor/runtime/server if the engine needs them.

## Consequences

- A new crate family (`lang-core`, a Rust backend, later a WASM backend, a
  host adapter, a conformance suite shared by the backends) and a
  `docs/dependency-rules.md` entry; no domain crate gains an outward edge.
- The first implementation PR starts with a grammar document and golden
  tests (as Substrate-Data's `docs/format.md` does), which also fixes the
  syntax.
- Hot reload of language code is a module swap (WASM) or a program
  rebuild (Rust backend), not statement-level patching.
- The first step is architecture, before any code: the concept trait, the
  pass pipeline, the port of Yog's analysis split into per-concept files,
  and the grammar document.
- Nothing is open for the owner in this ADR now; it stays Proposed until
  the grammar document lands.
- Engine freeze: while team/standards freezes the engine for waves 5-7,
  this ADR changes only on a branch.
