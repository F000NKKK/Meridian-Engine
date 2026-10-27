# ADR 022: the Meridian language, a compiled language

## Status

Proposed (2026-10-27, revised the same day). The owner asked for
"something like the language in Yog", then fixed the central point: the
language is **compiled, not a scripting language**. This revision replaces
the first draft, which put a reference interpreter first. Nothing below is
implemented. Order of work (owner): resources, hot reload, then this.

## Context

[ADR 017](017-mel-markup-and-code-behind.md) joins `.mel` markup to
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
   `build.rs`, as `mel-build` already does for markup. No runtime, native
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
  concept. The same diagnostics feed the build, hot reload and the `.mel`
  language server on Substrate's `lsp`.
- **Typed IR between front end and backends.** A backend takes the checked
  IR; a construct a backend lacks is a compile error, never a silent
  fallback.
- **Rust backend first.** It generates `.g.rs` modules like the markup
  glue (`mel_include!`-style), so a `.mel` `{x:Bind}` or event handler can
  name a function of the language the way it names a Rust method today.
  `Fixed`, `Handled` and the UI types map to the engine's own types.
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
  [ADR 018](018-ui-architecture.md) and `docs/mel-events.md`: handlers are
  named in markup and bound at compile time; a panic disables the handler
  and is reported with `file:line`.
- **Determinism.** A module marked `fixed` may use only `Fixed`, integers
  and host calls declared deterministic; the checker rejects `f32` and
  unordered iteration there.
- **Naming.** It is the Meridian language, not a "script language". The
  markup stays `.mel`, resources stay `.mres`; the language's own file
  extension is open (see Consequences).

## Consequences

- A new crate family (`lang-core`, a Rust backend, later a WASM backend, a
  host adapter, a conformance suite shared by the backends) and a
  `docs/dependency-rules.md` entry; no domain crate gains an outward edge.
- The first implementation PR starts with a grammar document and golden
  tests (as Substrate-Data's `docs/format.md` does), which also fixes the
  syntax.
- Hot reload of language code is a module swap (WASM) or a program
  rebuild (Rust backend), not statement-level patching.
- Open for the owner: the file extension (proposed `.mer`, markup `.mel`
  and resources `.mres` unchanged; the alternative is `.mel` for both,
  told apart by the root) and Rust-like (default) or C#-like syntax.
- Engine freeze: while team/standards freezes the engine for waves 5-7,
  this ADR changes only on a branch.
