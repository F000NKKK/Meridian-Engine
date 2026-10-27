# Segment: team/ui-ux

In-engine UI (HUD, menus, world-space panels, text) and the UX of the
editor / Meridian IDLE. This note is the segment's ownership map and
backlog; architecture decisions still go to `docs/adr/`, status to
`roadmap.md`.

## Owns

| Area | Where | State |
|---|---|---|
| UI rendering vocabulary (screen-space = `Scene2D` with pixel `Camera2D`, world-space = `Renderable3D` with `billboard`/`always_on_top`) | `meridian-graphics-core` (`scene.rs`), [graphics-design.md](../graphics-design.md) "UI is a role, not a third scene kind" | Design accepted; `Scene2D`/`Sprite`/`Camera2D` exist; `always_on_top` and blend modes not acted on by the submission bridge yet |
| Widgets, layout, focus, input routing | `meridian-ui-core`, [ADR 018](../adr/018-ui-architecture.md) | Element tree, typed properties, layout, hit testing, focus and input routing done; no controls or styles yet |
| Text: fonts, shaping, glyph atlas | `meridian-font-core`, [ADR 023](../adr/023-own-font-stack.md) | TrueType parser and outlines done; rasterizer, atlas, shaping and text in `ui-render` not yet |
| Pointer/keyboard input feeding UI | `meridian-platform-core::InputState` (consumer, not owner) | Key/mouse state exists; UI has its own `InputEvent`; the `platform-core` adapter that fills it is not written; no IME composition, no gamepad |
| Editor / IDLE UX (inspector, scene tree, viewport gizmos) | Meridian IDLE app (owned by **team/ide**); engine side through the editor protocol in `docs/data-formats.md` / substrate-data | We own UX requirements and engine-side hooks, team/ide owns the app |
| Tree to `Scene2d` | `meridian-ui-render`, [ADR 021](../adr/021-ui-extraction-and-extensibility.md) | `Background` fills extracted; text, borders, images, custom paint not yet; no sprite renderer consumes `Scene2d` |

## Boundaries

- `ui-core` (tier 1) depends on `foundation` and `memory-core` only. A
  later adapter above `graphics-core` and `platform-core` turns the
  laid-out tree into `Scene2D`/`Scene3D` renderables and feeds it
  `InputState`. Nobody calls `graphics-driver` from UI code and nothing
  adds a UI pass to `graphics-core`.
- `ecs-core` stays UI-agnostic. If widgets live in the ECS, the
  components are defined in `ui-core`, not in `ecs-core`.
- Layout math uses `gac-core` (`Motor3` frames, `Aabb`) instead of a
  parallel 2D math stack (ADR 001).
- Composition of the UI view into the frame (`[Scene3D, Scene2D-as-UI]`)
  is orchestration and belongs in `engine-core`/`sdk`.
- Text asset decoding (font bytes → glyph outlines) goes to
  `asset-core`; atlas lifetime goes through `resource-core` handles.

## Neighbours

- **team/rendering** — `always_on_top`, alpha blending, glyph atlas
  texture upload, render order of the UI view.
- **team/core** — input event stream in `platform-core`, `Runtime::tick`
  frame composition.
- **team/scripting** — UI markup in `.mel` (ADR 015 DSL) and bindings
  from scripts to widgets.
- **team/data** — serialised UI/screen files and the editor protocol.
- **team/ide** — Meridian IDLE app; we hand over UX specs and the
  engine side of the viewport.

## Prior art to reuse, not copy

`Yog-Mod-Loader/rust/crates/yog-ui` (retained-mode UI, flexbox +
WPF-style grid layout, focus, scroll, textbox, themes) and `yog-font`
(fonts as drawings, shaping, rasterisation). They are tied to Yog types,
so the useful part is the layout model and API shape. Whether anything
gets extracted to a neutral shared crate is a decision for the ADR.

## Direction from the owner

2026-10-27: UI and its `.mel` markup follow the **WPF model** — retained
element tree, layout panels (`Grid` with rows/columns in pixels/`Auto`/`*`,
`StackPanel`, `DockPanel`), styles/templates, data binding — modelled on
how `yog-ui` and Yog IDLE already do it. The UI-architecture ADR starts
from this, not from an open choice between retained and immediate mode.
Markup syntax is owned by team/scripting: ADR 017 (`.mel` markup and
code-behind, branch `claude/project-thread-h5kbvo`) — `x:Class`/
`x:ViewModel`, `x:Name`, `x:Key`, dotted property elements and attached
properties (`Grid.Row`), compiled `{x:Bind ...}` and `{StaticResource}`,
`foo.mel` → `foo.g.rs` via `mel_include!`. The UI-architecture ADR maps
that syntax onto the element model; ui-ux owns the runtime
(`meridian-ui-core`), scripting owns syntax, `dsl-core` and codegen.

## Principle: tool, not shackles

2026-10-27, owner: give developers the freedom Unity and UE5 give. Markup
optional, custom elements/draw/input via public traits, interception,
overridable styles, raw `Scene2d` escape hatch, no sealed types. Rules in
ADR 021 §6.

## Backlog

In order; nothing here starts without a task from the owner.

1. ~~**ADR: UI architecture**~~ — done as ADR 018 (proposed): standalone
   retained tree, measure/arrange, binding contract. Styles and input
   routing are listed there as not decided.
2. **ADR: text rendering** — font source formats, shaping scope,
   bitmap/SDF/MSDF atlas, where decoding lives.
3. **Rendering prerequisites** (with team/rendering) — honour
   `Material::always_on_top` and `AlphaBlend`; sprite alpha blending
   in the UI view.
4. **Input prerequisites** (with team/core) — text/IME input and an
   ordered input event stream in `platform-core`, plus a "consumed by
   UI" flag for gameplay.
5. **`meridian-ui-core` next steps** — skeleton is done (tree, layout,
   hit testing). Markup literal parsing is done
   (`PropertyType::parse`). Focus and input routing are done too. Left: `Scene2D`
   extraction adapter, text rendering, controls.
6. **Base widgets** — panel, label, button, image, scroll view, text
   box; theme/style tokens.
7. **Example** — HUD overlay over `magic_figures` or `physic_figures`
   through `./build.sh run`.
8. **World-space UI** — billboarded panels, health bars.
9. **UI markup** — tags in the `.mel` DSL (with team/scripting).
10. **Editor UX spec** for Meridian IDLE (with team/ide) — viewport
    gizmos, inspector driven by `Shape` descriptions, scene tree.
11. **Accessibility** — UI scale, contrast themes, keyboard-only
    navigation, screen-reader metadata later.
