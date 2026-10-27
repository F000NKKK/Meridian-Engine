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

Ordered by dependency. The model is decided in
[ADR 017](../adr/017-mel-markup-and-code-behind.md) (WPF-style markup plus
code-behind, compiled `{x:Bind}` bindings, Substrate-Data literal grammar);
the prior art to copy from is Yog-Mod-Loader's `.yui` -> `.g.rs` pipeline
(`yog-ui`, `yog-kit`) and Yog-IDLE's `docs/yui.md`/`mvvm.md`/`yres.md`.

1. ~~ADR 017: `.mel` behavior model~~ — done.
2. ~~Source spans in `dsl-core`~~ — done: `Span` (offset, line,
   column) on every `Element`, `Attribute` and `ParseError`.
3. ~~Grammar growth in `dsl-core`~~ — done: `x:` prefix, dotted names
   and property elements, markup extensions parsed into `MarkupValue`,
   typed `ParseErrorKind`; a tag built straight from markup rejects an
   extension value with a clear error (`text_attr`).
4. **Literal grammar.** Validate plain attribute values against
   Substrate-Data string rules (`@kind:id`, `@@`, error on other `@`),
   reusing `substrate-data`'s classifier (`typed::classify`, in progress
   on `team/data`'s side) rather than re-implementing the grammar.
5. ~~Generator crate + `mel_include!`~~ — done: `meridian-mel-build`
   (`compile_dir` from `build.rs`) writes `OUT_DIR/mel/<name>.g.rs` with
   `SOURCE`, a `Name` enum per `x:Name`, `Class`/`ViewModel` aliases,
   `build` and `find`, and rejects bad directives and names at build
   time; `magic_figures` uses it. Still open: **event dispatch**
   (`on_*` attributes -> code-behind methods), waiting for an engine
   event source to raise them, and porting `physic_figures` once markup
   hot reload (item 8) keeps run-time editing.
6. **`{x:Bind}` expression compiler.** Done, part 1: the Yog `x:Bind`
   subset (members, calls, `Type::path`s, literals, `!`, `==`, `!=`,
   `?.`) is parsed into `BindingExpr` and rendered as Rust over `vm`;
   `Mode`/`BindBack`/`Fallback` are validated (an `x:ViewModel` and an
   `x:Name` are required, `TwoWay` needs a member path or `BindBack`,
   `Fallback` needs a `?.`); the generated `type_check_bindings` makes
   `rustc` check every binding against the view model, under a
   `// @mel file:line:column` marker. Part 2, once `team/ui-ux` lands the
   element/property model: generated `apply_bindings` (OneWay/OneTime)
   and write-back from the target's change list (TwoWay), and accepting
   `{x:Bind}` attributes in built tags (today `text_attr` rejects them,
   so `build` fails for a document that has bindings).
7. **Resources.** `Resources` + `x:Key` in markup and an external
   resource file (Yog's `.yres` analogue, in Substrate-Data text) with
   generated typed accessors, referenced as `@res:Set.name`.
8. **Hot reload of markup.** Re-parse changed `.mel`/resources at run
   time and rebuild the affected scene part; code-behind changes still
   need a rebuild.
9. **Host API and script language.** One trait-based host API for all
   code-behind backends; then the own `.mel` script language
   (domain-blind lexer/parser/interpreter, `Fixed` path for fixed-step
   logic); then WASM.
10. **Tooling (with `team/ide`).** Diagnostics/symbols for a `.mel`
    language server on Substrate's `lsp`; a designer like Yog-IDLE's
    later; DAP once the interpreter can step.

## Split with other segments

- `team/ui-ux` owns the UI element, layout and binding runtime (future
  `meridian-ui-core`); this segment owns the `.mel` syntax, parser and
  generator that UI screens use too.
- `team/data` owns `.mscene` and the Substrate-Data adapter (ADR 016);
  `.mel` shares its literal grammar and is not compiled into `.mscene`.
- `team/ide` owns the editor; this segment provides diagnostics and
  symbols.
