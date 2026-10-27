# `.mel` events and commands

Contract between the markup side (`meridian-mel-build`, team/scripting)
and the event source (`meridian-ui-core`, team/ui-ux). It follows the
WinUI 3 / MVVM model the owner chose (ADR 017, ADR 018): handlers are
view-model methods bound with `x:Bind`, events are routed, and code-behind
handlers stay optional. Implemented: the event attributes, `dispatch` and
`Handled`/`IntoHandled` (ui-core); not yet: tunneling and commands. The event source (`UiEvent`, `UiTree::drain_events`, bubbling along
`UiTree::ancestors`) exists since ui-core #35.

## Markup

An attribute whose name is an event of the element, bound with a
plain `x:Bind` member path, is a handler:

```xml
<Button x:Name="save" click="{x:Bind save}" />
<TextBox x:Name="name" key_pressed="{x:Bind on_key(e)}" text_input="{x:Bind rename(e)}" />
```

- The right-hand side is a method of the `x:ViewModel`, written as in WinUI
  `x:Bind` event binding and named as in Rust like every other `x:Bind`:
  `method` or `method()` calls `fn save(&mut self)`, `method(e)` passes the
  event, `fn on_key(&mut self, event: &UiEvent)`. Either may return `()` or
  `Handled`. Rust checks the signature against the view model (ADR 017).
- An attribute that is a property of the element stays a property: the
  event on `TextBlock`'s `text` property is spelled `text_input`.
- The element needs an `x:Name` (the handler is keyed by its `NodeId` from
  `Names`).
- No `{x:Bind}` mode applies; `Mode`, `BindBack` and `Fallback` on an event
  attribute are errors.
- Code-behind may also subscribe by hand: `Handlers` below is plain data,
  so markup stays optional (ui-ux's rule: only plain `UiTree` calls).

## Generated code

For a document with handlers the generator adds

```rust
pub fn dispatch(vm: &mut ViewModel, names: &Names, tree: &UiTree, events: &[UiEvent]);
```

For each event, in order, it builds the route and calls the handlers of
the elements on it that registered for that event's tag:

- **Bubbling** (default): target first, then `tree.ancestors(target)`
  nearest first. Events that do not bubble (enter, leave, focus, per
  ui-core) visit the target only.
- **Tunneling** (`preview_*` tags, later): root first, down to the
  target, before the bubbling pass of the same input.
- A handler returning `Handled::Yes` stops the route; `fn(&mut self)`
  handlers count as `Handled::No` (WinUI's `e.Handled = true`, as a typed
  return instead of a flag).

The host drains events once per frame and calls `dispatch`; the view model
then reaches the tree through the usual `apply_bindings`. No global event
bus (CLAUDE.md: no singleton managers).

## Commands

`command="{x:Bind save_command}"` on a `Button`-like element binds an
`ICommand` equivalent: the view model's `Command` has `execute(&mut self)`
and `can_execute(&self) -> bool`; the generator applies `can_execute` to
the element's `enabled` property in `apply_bindings` and runs `execute`
on `click`. Deferred until ui-core has `enabled` and a `Button`.

## What ui-ux provides

1. A data-free tag for each event: `UiEventKind::tag() -> UiEventTag`
   (`Click`, `PointerPressed`, ...), with `UiEventTag::bubbles()` and
   `UiEventTag::from_attribute(&str)` (`"click"`, `"pointer_pressed"`)
   as the one place the attribute names live. The generator never
   matches on event names itself (no stringly-typed infrastructure).
2. Room for extension tags later (ui-ux's "tool, not shackles" rule): the
   dispatcher treats the tag as opaque and only asks `bubbles()`.
3. The `preview_*` tags when tunneling is wanted.

## What markup provides

Parsing and checking of the event attributes (`InvalidMarkup` at the
`.mel` line), `Handlers` in the generated module, `dispatch`, tests
against the real `UiTree`, and the `Handled` return type.
