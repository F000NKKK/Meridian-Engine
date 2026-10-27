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
   (done for bubbling: `click="{x:Bind save}"` -> view-model methods via
   the generated `dispatch`, `Handled`; contract in
   [mel-events.md](../mel-events.md); tunneling and commands still open),
   and porting `physic_figures` once markup
   hot reload (item 8) keeps run-time editing.
6. **`{x:Bind}` expression compiler.** Done. The Yog `x:Bind` subset
   (members, calls, `Type::path`s, literals, `!`, `==`, `!=`, `?.`) is
   parsed into `Expr` and rendered as Rust over `vm`;
   `Mode`/`BindBack`/`Fallback` are validated (an `x:ViewModel` and an
   `x:Name` are required, `TwoWay` needs a member path or `BindBack`,
   `Fallback` needs a `?.` and may not contain one). A bound attribute is
   resolved against `meridian-ui-core`'s registry at build time
   ([ADR 018](../adr/018-ui-architecture.md) section 5), so an unknown
   property is an error at its `.mel` line. The generated module gets a
   `Names` struct (a `NodeId` per `x:Name`), `apply_one_time_bindings`,
   `apply_bindings` (view model -> `tree.set`) and `write_back_bindings`
   (user edits from `tree.drain_changes()` -> view model or `BindBack`),
   each statement under a `// @mel file:line:column` marker, so `rustc`
   checks every binding against the view model. The view-model dirty
   flag that decides *when* `apply_bindings` runs is the caller's for now.
   A document whose root is a `ui-core` element (`Border`, `StackPanel`,
   `DockPanel`, `Grid`, `TextBlock`) also gets `build_ui(tree)`, which
   creates the elements in markup order, sets plain attributes through
   `tree.set_literal` (parsed by `PropertyType::parse`, `@@` escapes `@`)
   and returns the root with `Names`. Unknown tags, unknown properties,
   property elements and non-`{x:Bind}` extensions are build errors with
   their `.mel` line. Still open: references (`@kind:id`, `@res:`) in
   attributes, property elements (`<Grid.RowDefinitions>`), and accepting
   `{x:Bind}` in tags built by `build_scene` (today `text_attr` rejects
   them).
7. **Resources.** Done for values: `.mres` sets and `@res:Set.name`, plus
   `<Owner.Resources>` blocks with `x:Key` and `{StaticResource Key}`
   resolved at build time, nearest block first and `Set.name` falling back
   to the `.mres` entry (format and rules in
   [mel-resources.md](../mel-resources.md)). Still open: culture variants
   and run-time language switching, `{name}` placeholders,
   file/image/sound entries, styles and templates, `{DynamicResource}`.
8. **Hot reload of markup.** First step done: `LiveUi` (mel-build)
   takes a re-parsed `UiNode` and, when the document has the same
   elements, `x:Name`s and children, sets the attributes that changed (or
   resets removed ones to the property default) with plain `UiTree`
   calls, so every `NodeId` and `Names` stays valid. A different shape
   answers `Reload::Restructured` and leaves the tree alone; the host then
   rebuilds with the compiled `build_ui`. Still open: the host loop that
   watches the files and feeds `.mres` changes in (async file watching,
   ADR 009, belongs in the host, not in mel-build), `x:Bind` expressions
   (they are compiled code-behind, so a changed binding needs a rebuild),
   and subtree rebuilds that keep unchanged `NodeId`s.
9. **Host API and script language** (options compared against Yog's
   `.ys` in [ADR 022](../adr/022-mel-script-language.md)). One trait-based host API for all
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
