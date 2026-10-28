//! A value shared between the stages that touch it.

use std::sync::{Arc, Mutex, MutexGuard};

/// A handle to one piece of application state that several stages touch.
///
/// The composition root creates the state and hands a clone of the handle
/// to each stage that needs it, so every stage names exactly what it uses
/// and the runtime owns none of it. A poisoned lock is recovered: a
/// panicking stage must not wedge every later frame.
#[derive(Debug)]
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Shared<T> {
    /// Wraps `value` in a new shared handle.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    /// Locks the value. Blocks while another stage holds it; a stage that
    /// depends on that one never contends, since the dependency already
    /// finished and dropped its guard.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
