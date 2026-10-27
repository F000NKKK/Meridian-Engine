# ADR 022: the `.mel` script language

## Status

Proposed (2026-10-27). Written because the owner asked for "something
like the language in Yog" and for the options to be compared first.
Nothing below is implemented. Order of work (owner): resources, hot
reload, then this.

## Context

[ADR 017](017-mel-markup-and-code-behind.md) joins `.mel` markup to
code-behind in Rust, compiled with the game. That covers engine code and
trusted game logic. It does not cover logic that must be written, changed
and shipped without rebuilding the engine: mods, designer scripts, live
tuning, and gameplay that runs in the fixed-step deterministic path
([ADR 008](008-fixed-point-determinism.md)).

Yog-Mod-Loader already has such a language (`.ys`, with `.yns` as its
patch dialect). What it does, from its README, `docs/design/ys.md` and
the `yog-script-*` crates:

- One front end (`yog-script-analyze`: lexer, macros, parser, resolver,
  type and borrow checker) with one diagnostics type shared by the CLI,
  the build and the editor.
- A tree-walking interpreter (`yog-script-run`) that defines the
  semantics and runs build-time macros and `test` blocks, and a shipped
  backend that emits Java (`yog-script-java`). Unsupported constructs
  fail loudly (`YS0032`) instead of miscompiling.
- A host surface that is the project's own API (objects, services,
  events), not the game's; game classes are reachable only from the
  patch dialect. Events have a cancelling `Pre` and an observing `Post`.
- Typed ids, bound at build time to declarative files; a name-constants
  crate (`yog-script-ids`) so the analyzer and the backends agree.
- A C#-like surface with Rust's model on top (ownership, borrow checker
  over a GC, `Result`, macros, units, `saved`/`synced` state, sides as
  types). A panic in a handler disables that handler, not the game.
- Hot reload and the debugger exist for native Rust mods, not for
  scripts; there is no WASM, no step budget, and no determinism story
  (plain `i32`/`f32`).

What is Minecraft-specific there (sides, versions, patches, content
kinds, JVM) does not carry over. What is language-sized (the full C#
surface, Rust-grade borrow checking) is more than a small engine needs.

## Options

1. **Yog-sized language, compiled to a host.** Full surface, static
   types, borrow checking, transpile to Rust or bytecode. Strongest
   guarantees and speed; years of work, and it duplicates a compiler
   for a team that also builds an engine.
2. **Small own language with a reference interpreter** (this ADR's
   proposal). Static types over a deliberately small surface, an
   interpreter that defines the semantics, a `Fixed` numeric path, and
   backends added later (WASM first). Cost: a lexer, parser, checker and
   interpreter to own; benefit: determinism, hot reload and tooling are
   designed in, and the language stays domain-blind.
3. **Embed an existing language** (Rhai, Lua, JS). Fastest start. No
   fixed-point path, no typed host ids, diagnostics and LSP are someone
   else's, and hot reload and sandboxing follow the library's model.
4. **WASM only.** Any language compiles to it; the host API is the
   contract. Best isolation and a good end state, but it needs a
   toolchain on the author's machine and gives no in-engine editing,
   hot reload at statement level or `.mel`-adjacent authoring.

## Decision

Option 2, with option 4 as a later backend rather than an alternative:
the interpreter is the reference semantics, and a WASM backend must agree
with it, as in Yog ("one semantics, two runners").

The language, in the order it is built:

- **Domain-blind core.** A `script-core` crate with lexer, parser,
  checker and interpreter that names no engine concept. Types:
  `bool`, `i32`/`i64`, `f32`, `Fixed`, `string`, structs, enums with
  data, `Option`/`Result`, arrays. No GC-visible aliasing: values are
  owned or passed by `in`/`out`, so there is no borrow checker to build.
- **Host API as ports** (ADR 019). The script sees traits declared by the
  domain crates (UI, scene, input, time); a host crate adapts them. Ids
  are typed (`NodeId`, resource keys) and checked when the script is
  checked; no stringly-typed host calls.
- **Events and commands** follow the WinUI 3 model from
  [ADR 018](018-ui-architecture.md): a handler is a method named in
  markup (`{x:Bind Save}`), bound at check time; routed events reach it
  through the `UiTree`. Handlers run to completion; a panic disables that
  handler and is reported with `file:line`.
- **Determinism.** A script marked `fixed` may use only `Fixed`, integers
  and host calls declared deterministic; the checker rejects `f32` and
  unordered iteration there.
- **Diagnostics.** One diagnostic type with spans and stable codes,
  shared by the build, hot reload and the `.mel` language server on
  Substrate's `lsp`. Constructs the interpreter lacks are errors, not
  silent fallbacks.
- **Hot reload.** A changed script is re-checked and swapped at a frame
  boundary; state lives in host-owned structs, not in the script, so a
  swap keeps it. The markup side is already specified
  (`LiveUi`, [segments/scripting.md](../segments/scripting.md) item 8).
- **Limits.** A step budget per handler call from the start, so a script
  cannot stall a frame.

Not copied from Yog: the C# surface, borrow checking over a GC, sides as
types, content kinds, versions, the patch dialect.

## Consequences

- A new crate family (`script-core`, a host adapter, tests including a
  conformance suite shared with later backends) and a dependency-rules
  entry; no domain crate gains an outward edge.
- The decision on file extension and exact syntax is deferred to the
  first implementation PR, which must start with a grammar document
  and a set of golden tests (as `docs/format.md` does for Substrate-Data).
- Open question for the owner: C#-like or Rust-like syntax. The default
  is Rust-like, matching the engine and the team's Rust-first code;
  expression-bodied methods and string interpolation are cheap to add if
  C#-like feel matters more.
- Engine freeze: if team/standards freezes the engine for waves 5-7,
  work here pauses at the grammar document.
