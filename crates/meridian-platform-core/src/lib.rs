//! OS abstraction: window, input, filesystem, time, threading and dynamic library loading.
//!
//! Layer: infrastructure (adapter); bounded context: platform
//! (`craftbook.toml`, docs/context-map.md).
//!
//! One module per concern, re-exported flat from here: [`capabilities`]
//! (hardware/device detection and the `*-driver` reporting contract),
//! [`window`] (OS window + event loop), [`input`] (engine-owned
//! keyboard/mouse vocabulary), [`time`] (frame clock),
//! [`dynamic_library`] (plugin/hot-reload loading).
//!
//! Design notes: docs/notes/meridian-platform-core.md (`lib`).
#![doc = include_str!("../README.md")]
#![expect(
    unsafe_code,
    reason = "dynamic_library wraps the OS loader (dlopen/LoadLibraryW) by FFI"
)]

pub mod capabilities;
pub mod dynamic_library;
mod event_runner;
pub mod input;
mod key_map;
pub mod time;
pub mod window;

pub use capabilities::{BackendCapabilities, CpuCapabilities, GpuCapabilities, detect_cpu_threads};
pub use dynamic_library::{DynamicLibrary, DynamicLibraryError};
pub use input::{InputState, KeyCode, MouseButton};
pub use time::{Clock, Time};
pub use window::{AppHandler, Window, WindowError, run_windowed_app};
