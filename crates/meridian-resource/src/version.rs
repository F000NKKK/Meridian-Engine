//! [`Version`]: how many times a resource was reloaded.

/// Monotonically increasing version, bumped whenever a resource is reloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(
    /// The counter.
    pub u32,
);

impl Version {
    /// The version after this one. Wrapping is intentional: a resource
    /// reloaded `u32::MAX` times is already a bug worth finding elsewhere,
    /// not a reason to panic here.
    #[must_use]
    pub fn next(self) -> Version {
        Version(self.0.wrapping_add(1))
    }
}
