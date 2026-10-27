# Segment: scripting (`team/scripting`)

Ownership note and backlog for the scripting segment: `.meui` UI markup
(formerly `.mel`), `.mres` resources, and the compiled Meridian language
whose files use `.mel` ([ADR 022](../adr/022-meridian-language.md)). This file tracks *who owns what and what is
next*; permanent decisions go into ADRs, implementation status into
[roadmap.md](../roadmap.md).

## What exists today

`.meui` is currently only the declarative scene DSL from
[ADR 015](../adr/015-extensible-scene-dsl.md). There is no script
execution anywhere in the workspace yet.

| Piece | Where | Role |
|---|---|---|
| Markup parser | `crates/meridian-markup` (`parse`, `Element`, `ParseError`) | Domain-blind `<Tag attr="v">` tree; no text nodes, byte-offset errors only |
| Typed tags | `crates/meridian-markup/src/tag.rs` (`DslTag`, `DslRegistry`, `BuiltNode`) | Name -> builder registry, fails closed on unknown tags |
| Tag macro | `crates/meridian-markup-macros` (`#[dsl_tag(name = "...")]`) | Implements `DslTag` for a plain struct; used only via `meridian_sdk::dsl_tag` |
| Built-ins | `crates/meridian-sdk/src/dsl.rs` (`Scene`/`Entity`/`Mesh`/`Material`/`RigidBody`/`Transform`, `default_registry`, `build_scene`) | Ordinary `#[dsl_tag]` structs, no privilege over game tags |
| Samples | `examples/assets/scenes/{physic_figures,magic_figures}.meui` | `magic_figures` shows game-defined tags (`Glow`/`Orbit`/`Audio`) |

Per-frame behavior (orbit/spin, fixed-step accumulator) is still plain Rust
in the examples; the DSL describes composition only.

## Ownership and boundaries

The segment owns `meridian-markup`, `meridian-markup-macros`,
`meridian_sdk::dsl`, the `.meui` samples, and every future language crate.
It follows the workspace rules in [dependency-rules.md](../dependency-rules.md):

- The language core (lexer, parser, checker, IR; backends sit beside
  it) stays domain-blind, like `markup`: it never names ECS, physics or graphics
  types. Engine bindings live in an adapter crate above it.
- No global script manager or singleton VM ([ADR 003](../adr/003-no-global-managers.md));
  a script runtime is a value owned by whoever composes it (`runtime-app`
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
[ADR 017](../adr/017-meui-markup-and-code-behind.md) (WPF-style markup plus
code-behind, compiled `{x:Bind}` bindings, Substrate-Data literal grammar);
the prior art to copy from is Yog-Mod-Loader's `.yui` -> `.g.rs` pipeline
(`yog-ui`, `yog-kit`) and Yog-IDLE's `docs/yui.md`/`mvvm.md`/`yres.md`.

1. ~~ADR 017: `.meui` behavior model~~ — done.
2. ~~Source spans in `markup`~~ — done: `Span` (offset, line,
   column) on every `Element`, `Attribute` and `ParseError`.
3. ~~Grammar growth in `markup`~~ — done: `x:` prefix, dotted names
   and property elements, markup extensions parsed into `MarkupValue`,
   typed `ParseErrorKind`; a tag built straight from markup rejects an
   extension value with a clear error (`text_attr`).
4. **Literal grammar.** Validate plain attribute values against
   Substrate-Data string rules (`@kind:id`, `@@`, error on other `@`),
   reusing `substrate-data`'s classifier (`typed::classify`, in progress
   on `team/data`'s side) rather than re-implementing the grammar.
5. ~~Generator crate + `mel_include!`~~ — done: `meridian-ui-mel`
   (`compile_dir` from `build.rs`) writes `OUT_DIR/mel/<name>.g.rs` with
   `SOURCE`, a `Name` enum per `x:Name`, `Class`/`ViewModel` aliases,
   `build` and `find`, and rejects bad directives and names at build
   time; `magic_figures` uses it. Still open: **event dispatch**
   (done for bubbling: `click="{x:Bind save}"` -> view-model methods via
   the generated `dispatch`, `Handled`; contract in
   [meui-events.md](../meui-events.md); tunneling and commands still open),
   and porting `physic_figures` once markup
   hot reload (item 8) keeps run-time editing.
6. **`{x:Bind}` expression compiler.** Done. The Yog `x:Bind` subset
   (members, calls, `Type::path`s, literals, `!`, `==`, `!=`, `?.`) is
   parsed into `Expr` and rendered as Rust over `vm`;
   `Mode`/`BindBack`/`Fallback` are validated (an `x:ViewModel` and an
   `x:Name` are required, `TwoWay` needs a member path or `BindBack`,
   `Fallback` needs a `?.` and may not contain one). A bound attribute is
   resolved against `meridian-ui-domain`'s registry at build time
   ([ADR 018](../adr/018-ui-architecture.md) section 5), so an unknown
   property is an error at its `.meui` line. The generated module gets a
   `Names` struct (a `NodeId` per `x:Name`), `apply_one_time_bindings`,
   `apply_bindings` (view model -> `tree.set`) and `write_back_bindings`
   (user edits from `tree.drain_changes()` -> view model or `BindBack`),
   each statement under a `// @mel file:line:column` marker, so `rustc`
   checks every binding against the view model. The view-model dirty
   flag that decides *when* `apply_bindings` runs is the caller's for now.
   A document whose root is a `ui-domain` element (`Border`, `StackPanel`,
   `DockPanel`, `Grid`, `TextBlock`) also gets `build_ui(tree)`, which
   creates the elements in markup order, sets plain attributes through
   `tree.set_literal` (parsed by `PropertyType::parse`, `@@` escapes `@`)
   and returns the root with `Names`. Unknown tags, unknown properties,
   property elements and non-`{x:Bind}` extensions are build errors with
   their `.meui` line. Still open: references (`@kind:id`, `@res:`) in
   attributes, property elements (`<Grid.RowDefinitions>`), and accepting
   `{x:Bind}` in tags built by `build_scene` (today `text_attr` rejects
   them).
7. **Resources.** Done for values: `.mres` sets and `@res:Set.name`, plus
   `<Owner.Resources>` blocks with `x:Key` and `{StaticResource Key}`
   resolved at build time, nearest block first and `Set.name` falling back
   to the `.mres` entry (format and rules in
   [meui-resources.md](../meui-resources.md)). Culture variants
   (`Strings.ru.mres`) and the run-time language switch through
   `HotDocument::set_culture` are done. Still open: culture-aware
   lookup from code, `{name}` placeholders,
   file/image/sound entries, styles and templates, `{DynamicResource}`.
8. **Hot reload of markup.** Done for the markup and resource side:
   `LiveUi` patches the attributes that changed into a built tree (plain
   `UiTree` calls, `NodeId`s and `Names` stay valid) and answers
   `Reload::Restructured` when the shape changed; `HotDocument` watches
   the `.meui` and the `.mres` files under a directory with a
   non-blocking, debounced `SourceWatcher::poll` that the host calls
   from its frame loop, re-reads them (a bad file is an error that
   leaves the tree alone) and, on a new shape, builds the document again
   (`Refreshed::Rebuilt`: new root and `x:Name`d nodes by `Names` field;
   `{x:Bind}` values are applied again by the host). The watcher polls
   file stamps, which is bounded local work, so it is synchronous (no
   tokio); an OS-notification source can replace it behind the same
   `poll`. Still open: a changed `x:Bind` expression is compiled
   code-behind and needs a rebuild of the program, rebuilds that keep
   unchanged `NodeId`s, and the runtime-app wiring that calls `refresh`.
9. **Host API and the Meridian language** (compiled, not a scripting
   language: [ADR 022](../adr/022-meridian-language.md), compared against
   Yog's `.ys`). One trait-based host API for all code-behind backends;
   then the language: a domain-blind front end and typed IR, a Rust
   source backend first (like `ui-mel`), a WASM backend second, a
   small compile-time evaluator for constants and tests, a `Fixed` path
   for fixed-step logic.
10. **Tooling (with `team/ide`).** Diagnostics/symbols for a `.meui`
    language server on Substrate's `lsp`; a designer like Yog-IDLE's
    later; DAP once there is a debuggable backend (the WASM one, or the Rust
    one through its own debugger).

## Split with other segments

- `team/ui-ux` owns the UI element, layout and binding runtime (future
  `meridian-ui-domain`); this segment owns the `.meui` syntax, parser and
  generator that UI screens use too.
- `team/data` owns `.mscene` and the Substrate-Data adapter (ADR 016);
  `.meui` shares its literal grammar and is not compiled into `.mscene`.
- `team/ide` owns the editor; this segment provides diagnostics and
  symbols.
