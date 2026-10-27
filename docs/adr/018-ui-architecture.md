# ADR 018: WPF-style retained UI runtime (`meridian-ui-core`)

## Status

Proposed (2026-10-27). The owner's direction ("do it like WPF", looking at
how Yog-IDLE and Yog-Mod-Loader's `yog-ui` do it) is settled; the
mechanics below are the first implementation of it and are open to
review. The element tree, properties, layout and hit testing are
implemented in `meridian-ui-core`, as are focus and input routing; styles,
templates, controls and rendering are not.

## Context

[graphics-design.md](../graphics-design.md) already decided that UI is a
*role* of scenes (a pixel-space `Scene2D` over the world, or billboarded
quads in a `Scene3D`) and that widgets, layout, text and input routing
belong to a separate `ui-core`. The roadmap lists "no UI framework" as a
gap (7). [ADR 017](017-mel-markup-and-code-behind.md) fixed the markup
side: `.mel` is declarative, behavior lives in a code-behind, the two meet
through `x:Name`, attached properties (`Grid.Row`) and compiled
`{x:Bind}`; `meridian-mel-build` generates `foo.g.rs`. That generator needs
a runtime to write into. This ADR is that runtime.

`yog-ui` (Yog-Mod-Loader) is the working precedent: a retained tree,
flexbox plus a WPF-style `Grid`, focus and scrolling, themes. Its layout
model carries over; its types (mod-loader ids, `yog-gfx` drawing) do not.

## Decision

**1. A retained element tree, not immediate mode.** `UiTree` is an arena
of elements in a `memory-core` `ResourcePool`, addressed by generational
`NodeId` (ADR 002 and 004: handles, not references; the tree is data).
A removed subtree's ids go stale and are never reused for a live node.
Structure edits are `create`, `append_child`, `remove`. The tree owns no
behavior; behavior is the code-behind's.

**2. Typed dependency properties.** An element's state is a set of
properties, each a `const Property<T>` (`properties::grid::ROW:
Property<u32>`, `properties::element::WIDTH: Property<Length>`). Reads
and writes are `tree.get(node, P)` / `tree.set(node, P, value)`, checked at
compile time; there is no string-keyed property access at run time.
Values are stored as a closed `PropertyValue` enum (the same form the
inspector/designer uses through `set_value`/`value_of`, with the kind
checked), and unset properties read as the property's default. Attached
properties (`Grid.Row`, `DockPanel.Dock`) are ordinary properties read by
the parent's layout. A write that does not change the value is a no-op.

**3. Two-pass layout, as in WPF.** `compute_layout` runs *measure*
(each element reports the size it wants given the room offered) then
*arrange* (each parent hands out slots). Common to every element: `Width`,
`Height`, `Margin`, `HorizontalAlignment`, `VerticalAlignment`,
`Visibility` (`Visible`/`Hidden`/`Collapsed`). Panels so far: `Border`
(padding, single child), `StackPanel`, `DockPanel` (`LastChildFill`),
`Grid` (`Px`/`Auto`/`*` rows and columns), `TextBlock` (sized through the
`TextMeasurer` port, so the font stack is a later, separate ADR). Layout
is in logical pixels, origin top-left, `y` down, matching the `Scene2D`
camera convention. Results are stored per node, relative to the parent.

**4. Hit testing.** `hit_test(tree, root, point)` returns the topmost
visible element; later siblings are on top and a parent clips its
children for hit purposes.

**5. The binding contract for generated code.** What `mel-build` can
rely on:

- `x:Name` becomes a `NodeId` the generated `Names` struct holds.
- A markup attribute `A` on element `X` resolves at build time to the
  constant `properties::<snake(X)>::<SCREAMING(A)>`, falling back to
  `properties::element::<SCREAMING(A)>`; `Owner.Name` resolves to
  `properties::<snake(Owner)>::<SCREAMING(Name)>`. A missing property is a
  compile error in the generated code, not a run-time lookup. (`properties::by_name`
  exists for tooling only.)
- `OneWay`/`OneTime`: generated code evaluates the expression against the
  view model and calls `tree.set(node, P, value)`; equal writes cost
  nothing and do not dirty layout.
- `TwoWay`/`BindBack`: user-driven writes go through
  `tree.set_by_user`, which journals a `Change { node, property, source:
  User }`. Generated code drains `tree.drain_changes()`, keeps the `User`
  entries it has bindings for, reads the value with `tree.get` and writes
  it to the view model (or calls the `BindBack` method). Code-driven
  writes are journaled as `Code` so a binding never echoes into itself.
- View-model change notification starts coarse (a per-view-model dirty
  flag re-applying its `OneWay` bindings) and gets finer only if a profile
  asks for it. That interface belongs to `team/scripting`'s generator and
  is not fixed here.
- Markup literals are parsed by the runtime, not the generator: the
  generator emits the attribute text and the property constant, and
  `PropertyType::parse(text)` turns `"Auto"`, `"4,2"` or `"2*"` into the
  typed value (`tree.set_literal(node, id, text)` does the same for
  tooling). Plain text follows the Substrate-Data `@` rule: `@@` is a
  literal `@`, any other leading `@` comes back as
  `LiteralError::Reference` for the caller to resolve.

**6. Input and focus.** The UI has its own input vocabulary
(`InputEvent`: pointer moved/pressed/released, `KeyPressed` with
`Modifiers`, `Text`), independent of any windowing backend; the adapter
above `platform-core` translates into it. `UiTree::handle_input(root,
event)` routes the event with the last layout and returns
`InputOutcome::Consumed` or `Ignored`, which gameplay reads to leave
UI-used input alone. Pointer events target `hit_test`'s result; elements
with `IsHitTestVisible = false` are transparent to the pointer. Keyboard
focus is the `Focusable` property plus `focus`, `clear_focus` and
`move_focus` (document order, wrapping; `Tab`/`Shift+Tab` do it); only an
element that is `Focusable` and `Visible` along its whole ancestor chain
takes focus, and a click focuses the nearest focusable ancestor-or-self.
Key and text events go to the focused element. The result is a queue of
`UiEvent { target, kind }` (`PointerPressed`/`Released`, `Click`,
`PointerEntered`/`Left`, `KeyPressed`, `Text`, `FocusGained`/`Lost`)
drained with `drain_events`; the code-behind (or generated glue) handles
them, and bubbling is the ancestor path (`UiTree::ancestors`) rather than
a handler registry in this crate. Removing a node clears any focus, hover
or press state pointing at it. No pointer capture, drag, gesture or
gamepad navigation yet.

**7. Boundaries.** `ui-core` depends on `foundation` and `memory-core`
only (see [dependency-rules.md](../dependency-rules.md)). It does not
depend on `graphics-core` or `platform-core`: a later adapter above both
walks the laid-out tree into `Scene2D` sprites and quads and feeds it
`InputState`. It does not depend on `ecs-core`: the tree is its own
structure, and nothing in the ECS knows about UI. `ui-core` stays
domain-neutral; game widgets are defined by the game.

## Not decided yet

- Styles, resource dictionaries, control templates, triggers and
  inheritance of property values down the tree.
- Tunnelling events, pointer capture and drag, directional (gamepad)
  focus navigation, `TabIndex`, IME composition (only finished text
  arrives as `Text`), `Enter`/`Space` activation (a control's job).
- Controls (`Button`, `TextBox`, `ScrollViewer`) and content model
  (`ContentControl`); more panels (`Canvas`, `WrapPanel`); `Min`/`Max`
  sizes; `Grid` row/column spans.
- Extraction to `Scene2D`/`Scene3D` and the draw-order contract; text
  rendering (own ADR).
- Whether a float/`Fixed` split is needed: layout is presentation, not
  simulation, so it stays `f32`.

## Consequences

- Scripting can generate against a stable, type-checked write target
  today; extending the property set is adding a `const` and a registry
  row.
- The closed `PropertyValue` enum keeps the inspector simple and the
  storage free of `dyn Any`, at the price that a new value kind is a
  change in this crate. Game-specific kinds will need an extension
  point once a concrete one exists.
- Layout reads properties through `get` per element per pass. That is
  fine for UI-sized trees; a column-per-property layout cache is possible
  later behind the same API.
