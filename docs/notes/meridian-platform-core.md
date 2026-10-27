# `meridian-platform-core` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

`Time`/`Clock` and `InputState` are pure state machines with no OS
dependency, so they need no external crate.
`Window` is real, backed by `winit` (see
[ADR 010](../adr/010-windowing-via-winit.md) for why: correct
cross-platform windowing turned out to be the same class of
multi-month, many-independent-bug-classes undertaking that justified
accepting `wgpu` over hand-written GPU FFI). [`run_windowed_app`] owns
the OS event loop (`winit` requires this — window creation and event
delivery only happen inside its callback-driven model, not on demand)
and translates `winit`'s own event types into this crate's existing,
`winit`-independent [`KeyCode`]/[`MouseButton`]/[`InputState`]
vocabulary — callers implement [`AppHandler`] and never see a `winit`
type. [`DynamicLibrary`] is real and stays on ADR 010's original
hand-written-FFI plan (unlike windowing, its scope really is small):
`dlopen`/`dlsym`/`dlclose` on POSIX, `LoadLibraryW`/`GetProcAddress`/
`FreeLibrary` on Windows, declared directly as `extern` blocks — no
external crate. Loading is a synchronous call deliberately: the OS
APIs have no async form, so there is nothing to await (a caller that
must not block wraps it in `tokio::task::spawn_blocking`, the same
escape hatch ADR 009 prescribes for `wgpu::Device::poll`).
