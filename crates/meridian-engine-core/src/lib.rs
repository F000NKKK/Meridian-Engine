//! Runtime: frame scheduler, event system, subsystem manager; ties every other crate into the main loop.

/// Owns subsystem instances and drives the frame loop.
#[derive(Debug, Default)]
pub struct Runtime;

/// Builds and runs the per-frame job graph across every subsystem.
#[derive(Debug, Default)]
pub struct FrameScheduler;

/// Workspace-wide event bus.
#[derive(Debug, Default)]
pub struct EventSystem;

/// Registry of active subsystems for the current `Runtime`. The only place
/// in the workspace allowed to know about every `*-core` at once — see
/// docs/dependency-rules.md rule 7.
#[derive(Debug, Default)]
pub struct SubsystemManager;
