//! A decoded sample buffer played from memory.

use super::{Repeat, SampleSource};

/// Mono samples held in memory, read front to back.
#[derive(Debug, Clone)]
pub struct MemorySource {
    samples: Vec<f32>,
    cursor: usize,
    repeat: Repeat,
}

impl MemorySource {
    /// A source over `samples`, positioned at the start.
    #[must_use]
    pub fn new(samples: Vec<f32>, repeat: Repeat) -> Self {
        Self {
            samples,
            cursor: 0,
            repeat,
        }
    }
}

impl SampleSource for MemorySource {
    fn read(&mut self, out: &mut [f32]) -> usize {
        if self.samples.is_empty() {
            return 0;
        }
        let mut written = 0;
        while written < out.len() {
            if self.cursor == self.samples.len() {
                if self.repeat == Repeat::Once {
                    break;
                }
                self.cursor = 0;
            }
            let run = (self.samples.len() - self.cursor).min(out.len() - written);
            out[written..written + run]
                .copy_from_slice(&self.samples[self.cursor..self.cursor + run]);
            self.cursor += run;
            written += run;
        }
        written
    }
}
