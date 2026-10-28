//! The port a voice pulls its mono samples from.

/// A pull source of mono samples: a decoded buffer, a streaming decoder, a
/// synthesizer. Declared here by the domain that consumes it (ADR 019);
/// asset and SDK crates implement it for their decoded tracks.
pub trait SampleSource: Send {
    /// Writes up to `out.len()` consecutive samples into the front of `out`
    /// and returns how many it wrote. Fewer than requested means the source
    /// has ended; a looping source never ends and always fills `out`.
    fn read(&mut self, out: &mut [f32]) -> usize;
}
