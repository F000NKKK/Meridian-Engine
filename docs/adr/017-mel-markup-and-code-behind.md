# ADR 017: `.mel` behavior via WPF-style code-behind and compiled bindings

## Status

Accepted (2026-10-27). Nothing below is implemented yet; see
[segments/scripting.md](../segments/scripting.md) for the backlog.

## Context

[ADR 015](015-extensible-scene-dsl.md) made `.mel` a declarative,
extensible markup for scene composition and left behavior out of scope:
per-frame logic (orbit/spin, fixed-step stepping) is plain Rust in the
examples. The roadmap's earlier direction was a Razor-style extension
that embeds script code inside the markup (`@expr`, `@{ ... }`).

Three things argued against Razor:

- The owner's direction is "do it like WPF": markup describes structure,
  a code-behind file holds the logic, and the two are joined by names,
  events and compiled bindings, never by code pasted into the markup.
- The workspace already has a working WPF-style precedent in the
  sibling projects: Yog-Mod-Loader's `.yui` screens (`yog-ui`, `yog-kit`)
  generate a `<name>.g.rs` (the counterpart of WPF's `MainWindow.g.cs`)
  that a hand-written code-behind pulls in, with `x:Name`, `x:ViewModel`,
  `{x:Bind ...}` and `Resources`/`Style`; Yog-IDLE edits them with a
  designer kept in sync both ways. Meridian IDLE will want the same.
- `team/data`'s [ADR 016](016-data-formats-on-substrate-data.md) uses the
  Substrate-Data literal grammar, where a leading `@` starts a reference
  (`@kind:id`) and `@@` escapes it. Razor's `@` would collide with it in
  every attribute value.

## Decision

**1. Markup plus code-behind, as in WPF.** A `.mel` document stays pure
markup. Its root may name a code-behind with `x:Class="path::Type"` (a
type with handler methods) or `x:ViewModel="Type"` (state plus logic,
bound from the markup). Elements are addressed with `x:Name`; events are
attributes naming a code-behind method (`on_collide="explode"`).
No code is embedded in the markup, and Razor-style `@{ }` is dropped.

**2. Generated glue, never committed.** A build-time generator turns
`foo.mel` into `foo.g.rs` in the build output (`OUT_DIR`), the way WPF
generates `.g.cs` and Yog generates `.g.rs`. It holds `ids`, typed
accessors for `x:Name` elements, the scene/view builder, binding update
code and event dispatch. The code-behind pulls it in with a
`mel_include!("foo")` macro. Type errors in bindings point at the `.mel`
line through `// @mel file:line` markers.

**3. Compiled bindings.** Markup extensions use braces, as in XAML:
`{x:Bind expr[, Mode=OneWay|TwoWay|OneTime][, BindBack=method][, Fallback=value]}`
and `{StaticResource Key}`. `expr` is a small expression language compiled
to Rust (field and method paths, calls, literals, `!`, `==`, `!=`, `?.`),
the same scope as Yog's `x:Bind`; anything larger belongs in a
code-behind method. Bindings are not reflection-based, so they cost no
more than the hand-written Rust they replace.

**4. One literal grammar with Substrate-Data.** Plain attribute values
follow Substrate-Data's string rules, as Yog-IDLE's `.yui`/`.yres` follow
Yog Data's: `@kind:id` is a reference (`@mesh:hero.mesh`,
`@res:Strings.title`), `@@` starts a literal `@`, any other leading `@` is
an error. `.mel` is not compiled into `.mscene`: `.mel` is authored
composition plus behavior wiring, `.mscene` is stored scene data
(ADR 016), and the shared grammar is what lets the two reference the same
assets and resources without a translation layer.

**5. Languages.** The first code-behind language is Rust. Logic that must
change without a Rust rebuild comes later through an own `.mel` script
language with a domain-blind interpreter (the role `yog-script-run` plays
for Yog Script), then WASM. Every backend binds through one host API, so
the markup and generated glue do not depend on which language a
code-behind is written in. Logic running in the fixed-step simulation
uses the deterministic `Fixed` path ([ADR 008](008-fixed-point-determinism.md)).

**6. Ownership.** `team/scripting` owns the `.mel` syntax, `dsl-core`,
the generator and the script backends. `team/ui-ux` owns the UI element,
layout and binding runtime (future `meridian-ui-core`); UI screens use
the same `.mel` syntax and generator.

## Consequences

- `meridian-dsl-core`'s grammar grows: the `x:` prefix, dotted names for
  property elements and attached properties (`Grid.Row`,
  `Entity.Components`), markup-extension values, and line/column spans
  for diagnostics and the editor. Its "no namespaces" rule in the crate
  doc changes to "only the `x:` prefix".
- New crates are planned: a build-time generator used from a
  `build.rs`, and later the script language core. Each is added to
  `docs/dependency-rules.md` and `scripts/check_dependency_rules.py` when
  it is created, not before.
- Hot reload splits in two: markup and resource changes reload at run
  time; Rust code-behind changes need a rebuild until a script backend
  exists.
- ADR 015's tag registry stays the extension mechanism for element types;
  this ADR only adds behavior wiring on top of it.
