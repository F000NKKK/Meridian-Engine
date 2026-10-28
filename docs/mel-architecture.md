# MEL architecture

MEL is the Meridian Engine Language ([ADR 022](adr/022-meridian-language.md)):
a compiled language whose files use `.mel`. This page is the architecture of
its compiler and how it meets the engine. The syntax is in
[mel-grammar.md](mel-grammar.md). Nothing here is implemented yet; the first
code is the lexer and parser with golden tests (step 1 below).

## Goals

- **Readable and maintainable.** Yog Script's analysis lives in a few very
  large files; MEL's does not. Every keyword, expression form, statement form
  and declaration form is **one concept**, and one concept is one directory.
- **No boilerplate to add a construct.** A new construct is a new directory
  plus one line in a registry. No edit to a giant `match`, token table or
  visitor.
- **One front end, two targets.** The Rust backend (primary) and the WASM
  backend consume the same typed IR and agree on semantics.
- **Domain-blind.** The compiler names no engine concept. The engine reaches
  MEL through host ports (ADR 019).

## Crates

Names follow ADR 026 (`meridian-<context>-<layer>`, context `mel`).

| Crate | Layer | Holds |
|---|---|---|
| `meridian-mel-domain` | domain | Lexer, parser, checker, ownership checker, typed IR, diagnostics, the compile-time evaluator. No I/O, no `async`, no globals. |
| `meridian-mel-app` | app | The pipeline (pass order), the concept registry's composition, project loading ports (`SourceReader`), the build entry used from `build.rs` and the CLI. |
| `meridian-mel-rust` | infra | Rust backend: typed IR to `.g.rs`. |
| `meridian-mel-wasm` | infra | WASM backend: typed IR to a module, host-call ABI, fuel metering. |
| `meridian-mel-host` | infra | Run-time loader for WASM modules: capability-scoped host API, frame-boundary swap, fuel limit. |

Edges point inwards: backends depend on `mel-domain`; `mel-domain` depends on
nothing in the workspace except the diagnostics kernel. The host API traits
(UI, scene, input, time) are declared by the domain crates that own them
(`ui-domain`, ...) and adapted by `mel-host` / the generated Rust, never by
`mel-domain`. `ui-mel` (the `.meui` generator) calls `mel-app` to compile the
`.mel` code-behind next to a `.meui`.

## Concepts: one directory each

```
meridian-mel-domain/src/
  lib.rs                 wiring only
  concept.rs             the traits a construct implements
  registry.rs            Registry: the concepts, built once and passed in
  syntax/                tokens, spans, the node arena (shared, small)
  ir/                    the typed IR: a small closed core (below)
  keyword/  let_decl/ ...
  expr/     literal/ binary/ call/ member/ lambda/ switch_expr/ interpolation/ try_expr/ ...
  stmt/     assign/ if_stmt/ foreach_stmt/ return_stmt/ ...
  decl/     class/ struct_decl/ record/ enum_decl/ interface/ component/ system/ macro_decl/ ...
  ownership/             borrow and move checking over the IR
  eval/                  compile-time evaluator (constants, macros, tests)
```

A concept directory has the same shape everywhere:

```
expr/binary/
  mod.rs      wiring: mod + pub use + the Concept impl
  lex.rs      its tokens (operator symbols)
  parse.rs    its Pratt rule (binding power, how to parse the right side)
  check.rs    its type rule
  lower.rs    its lowering to IR
  tests.rs    unit tests; golden files live under tests/golden/
```

`concept.rs` declares small traits, not one giant one:

- `Lexes`: contributes token kinds and their recognizers.
- `Parses`: contributes a prefix, infix or statement/declaration parse rule.
- `Checks`: gives the type/name rule for the nodes it parsed.
- `Lowers`: turns its checked node into IR.

A concept implements the traits it needs. `Registry` is built in the
composition root (`mel-app`) from a list of concepts and passed to the
pipeline explicitly; there is no global registry. Pratt parsing makes
operators data: an infix concept supplies its binding power, so the parser
loop never names an operator.

### Why the IR is small and closed

Syntax is open (registered concepts); the **IR is a closed core** of about
two dozen node kinds: constants, locals, calls, field access, branches,
loops, aggregates, enum construction and match on tags, move/borrow/drop,
host calls. Every concept lowers to that core ("desugaring"), so the two
backends match on the IR only and never learn a language construct. A new
syntax form usually adds no backend work. A construct a backend lacks is a
compile error naming the construct, never a silent fallback.

## Pipeline (orchestration)

`mel-app` owns the order; it names no concept:

```
SourceReader -> lex -> parse -> resolve names -> check types
  -> check ownership (move/borrow/lifetimes) -> expand macros (eval)
  -> lower to IR -> backend (rust | wasm)
```

Macro expansion repeats parse/resolve until no macro call is left (depth
limit), then the rest runs. Each pass is a small function over the arena that
asks the registry for the rule of a node's kind. Diagnostics are collected,
not thrown: a pass keeps going to report as many as it can, then the pipeline
stops before the next pass if any error was reported.

## Ownership

Rust's rules, written C#-style (`in`, `ref`, `out`, `move`; see the grammar).
There is no GC, because the primary target is Rust. The ownership checker
runs on the IR (after lowering to explicit moves and borrows), non-lexical,
with lifetime elision at signatures. The Rust backend then emits `T`, `&T`,
`&mut T` and `Drop` one to one, so a program the checker accepts is accepted
by `rustc`; a mismatch is a compiler bug, not a user error.

## Determinism

A module marked `fixed` may use only `Fixed`, integers and host calls declared
deterministic (ADR 008). The checker rejects `f32`/`f64` and unordered
iteration there. This is a type rule, so it costs nothing at run time.

## Backends

- **Rust (`mel-rust`).** Generates `.g.rs` from `build.rs`, as `ui-mel` does
  for markup. Generated code carries `// @mel file:line:col` markers so a
  `rustc` error points at the `.mel` line. The `.meui` code-behind pairing is
  here: a `.mel` class named by `x:Class` becomes the view model the `.meui`
  generator binds to.
- **WASM (`mel-wasm` + `mel-host`).** Same `.mel` syntax and IR, a module per
  file. The host gives a module only the capabilities granted to it (none by
  default; no filesystem or network unless granted), a fuel limit per call,
  and swaps a rebuilt module at a frame boundary. State lives in host-owned
  structs so a swap keeps it.

## Diagnostics and tooling

Codes are stable (`MEL0001`...), with spans; the same type feeds the build,
hot reload and the language server on Substrate's `lsp` (with `team/ide`).
A panic in a handler disables that handler and is reported at its `.mel`
line; the frame goes on.

## Testing

- **Golden tests** per concept: `tests/golden/<concept>/<name>.mel` with an
  `.expected` file for tokens, AST, diagnostics and generated Rust. Malformed
  input is an error, never a panic or an unbounded allocation.
- **Backend conformance:** one suite of programs run through both backends
  with identical expected results.
- Craftbook coverage targets apply (domain 90 %).

## Steps

1. `mel-domain`: syntax (tokens, spans, arena), lexer and parser for the
   phase-1 subset in the grammar, with golden tests. No checker yet.
2. Name resolution and type checking for that subset; diagnostics codes.
3. Typed IR, ownership checker.
4. `mel-rust` backend; `.meui` code-behind pairing in `ui-mel`.
5. Enums with data, `switch` exhaustiveness, `Result` and `try`.
6. Macros and the compile-time evaluator; `test` blocks.
7. `mel-wasm` and `mel-host`; capabilities and fuel.
8. Components and systems over `ecs`.

## Open points

- Whether `component`/`system` (Yog's strongest idea) belong in the language
  or stay a library over `ecs` plus macros. They are the last step on
  purpose.
- Sides (editor, runtime, server) as types, as Yog does for Minecraft's
  client and server. Not needed until the engine has a server role.
