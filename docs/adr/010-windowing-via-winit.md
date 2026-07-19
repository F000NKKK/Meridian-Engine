# ADR 010: Windowing via winit, not hand-written per-platform FFI

## Status

Accepted — supersedes the `Window`/`DynamicLibrary` "hand-written FFI"
plan recorded in earlier `docs/roadmap.md` revisions.

## Context

`platform-core::Window` has been a deliberate stub since this crate's
first pass, with the recorded plan being hand-written unsafe FFI
(per-platform window creation: X11/Wayland on Linux, Win32 on Windows,
Cocoa on macOS) — the same zero-external-dependencies default this
workspace applies everywhere except `wgpu` (see
[ADR](../roadmap.md) "Not yet decided" wgpu entry, `wgpu`'s own
justification: reimplementing Vulkan/DX12/Metal by hand is a multi-month
undertaking with three independent classes of driver bugs).

That plan was written when `Window` was still deferred and its actual
scope hadn't been stress-tested against what a real render-to-screen
example needs. Revisiting it: correct, real cross-platform windowing is
not "create an OS window" alone — it's window lifecycle (create, resize,
close, focus, minimize/restore), input event delivery (keyboard,
mouse, including modifier state and platform-specific key mapping),
HiDPI/monitor-scale handling, and IME/accessibility integration, each
with its own quirks per platform (X11 vs. Wayland alone are different
enough to be separate implementations, not a shared code path). Doing
this correctly by hand across X11 *and* Win32 *and* Cocoa is the same
shape of multi-month, multiple-independent-bug-classes undertaking that
justified accepting `wgpu` over hand-written Vulkan/DX12/Metal bindings —
the "these stay small enough to hand-roll safely" reasoning the original
plan rested on does not survive contact with what a real implementation
requires.

## Decision

**`winit`, at the latest version compatible with this workspace's
`wgpu` version, not hand-written per-platform FFI.** `platform-core::Window`
wraps a real `winit::window::Window` (behind an `Arc`, so it can be
shared with a render backend without cloning window state). `winit` is
the de facto standard windowing crate in the Rust ecosystem, already
what `wgpu`'s own examples and most Rust engines/frameworks build on, and
its `raw-window-handle` integration is exactly what `wgpu::Surface`
creation needs — accepting it doesn't just save the hand-written FFI
work, it removes an entire class of window/surface-handle plumbing bugs
this workspace would otherwise have to get right itself, across three
platforms, before a single pixel could reach the screen.

**`meridian-graphics-driver` stays winit-agnostic.** `Device::new_windowed`
takes `impl Into<wgpu::SurfaceTarget<'static>>` (a `wgpu`-defined bound
that `Arc<winit::window::Window>` already satisfies via `raw-window-handle`),
not a `winit`-specific parameter type — `graphics-driver` itself does not
depend on `winit` at all. This preserves the same "driver crate doesn't
know its caller's specific windowing choice" boundary
[ADR 005](005-driver-core-separation.md) establishes for every other
`*-driver`/`*-core` split, and means a future non-`winit` window source
(should one ever be needed) would only have to satisfy the same `wgpu`
bound, not a `meridian-graphics-driver`-specific interface.

**`platform-core`'s existing input vocabulary (`KeyCode`, `MouseButton`,
`InputState`) is unchanged.** These were already designed as an
engine-owned vocabulary independent of any specific windowing backend
(`InputState` is a plain pushed-state struct — `press_key`/`release_key`/
`set_mouse_position`/... — not tied to `winit`'s own event types). Real
`winit` event handling translates `winit`'s key/mouse events into calls
on that existing API; `KeyCode`/`MouseButton`/`InputState` themselves
needed no redesign for this decision, which is itself evidence the
original design was already winit-compatible even while `Window` was
still a stub.

**`DynamicLibrary` is unaffected — still hand-written FFI.** This ADR is
scoped to windowing specifically; `dlopen`/`LoadLibrary` for
`DynamicLibrary` stays the plan (it's genuinely small, no ecosystem-
standard crate the way `winit` is for windowing, and unlike windowing
its scope hasn't grown on closer inspection).

## Alternatives considered

- **Hand-written per-platform FFI (the original plan)** — rejected on
  revisiting scope; see "Context" above. Would have meant reimplementing
  a meaningful slice of what `winit` already gets right across three
  platforms, for a `Window` type that exists specifically to unblock
  real rendering — the least appropriate place in this workspace to
  accept subtle, hard-to-test-across-platforms bugs.
- **A different windowing crate (`sdl2`, platform-specific bindings via
  `x11rb`/`windows-rs`/`objc2` used directly)** — rejected: none is as
  broadly used alongside `wgpu` specifically as `winit`, and this
  workspace has no existing reason (audio, input, or otherwise) to prefer
  SDL2's broader scope over a windowing-focused crate.
- **Defer `Window` further, keep it a stub** — rejected: window/swapchain
  presentation is the concrete next milestone (a real render-to-screen
  example), and every other prerequisite (`graphics-driver`'s headless
  `wgpu` device, `gac-core`'s transform math, `graphics-core`'s camera/
  render-graph) is already real and waiting on exactly this.

## Consequences

- `platform-core` gains its first external dependency (`winit`), the
  same kind of deliberate, documented exception to zero-deps `wgpu`
  already established for `graphics-driver` — not a reversal of the
  general policy.
- `meridian-graphics-driver` gains real surface/swapchain support
  (`Device::new_windowed`, a `Surface` type) without gaining a `winit`
  dependency itself.
- A real windowed example (spinning cube, then more later) is unblocked;
  see [roadmap.md](../roadmap.md) for current status.
- `DynamicLibrary` remains a stub on the original hand-written-FFI plan —
  this decision does not touch it.
