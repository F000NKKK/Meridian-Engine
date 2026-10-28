//! An in-memory [`AudioSink`](crate::AudioSink) that keeps what it is given.

use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::AudioSink;

/// Records every pushed sample; never blocks, and reports room until `capacity`
/// samples are held.
#[derive(Debug)]
pub struct RecordingSink {
    capacity: usize,
    samples: Mutex<Vec<f32>>,
}

impl RecordingSink {
    /// A sink that has room for `capacity` samples.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            samples: Mutex::new(Vec::new()),
        }
    }

    /// Every sample pushed so far, in order.
    #[must_use]
    pub fn recorded(&self) -> Vec<f32> {
        self.held().clone()
    }

    /// Empties the sink, as a device draining its ring buffer would.
    pub fn drain(&self) {
        self.held().clear();
    }

    fn held(&self) -> MutexGuard<'_, Vec<f32>> {
        self.samples.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl AudioSink for RecordingSink {
    fn push_interleaved(&self, samples: &[f32]) {
        self.held().extend_from_slice(samples);
    }

    fn can_push(&self, sample_count: usize) -> bool {
        self.capacity.saturating_sub(self.held().len()) >= sample_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_in_order_and_drains() {
        let sink = RecordingSink::with_capacity(8);
        sink.push_interleaved(&[0.1, 0.2]);
        sink.push_interleaved(&[0.3]);
        assert_eq!(sink.recorded(), vec![0.1, 0.2, 0.3]);
        sink.drain();
        assert_eq!(sink.recorded(), Vec::new());
    }

    #[test]
    fn room_shrinks_as_samples_arrive() {
        let sink = RecordingSink::with_capacity(4);
        assert!(sink.can_push(4));
        sink.push_interleaved(&[0.0; 3]);
        assert!(sink.can_push(1));
        assert!(!sink.can_push(2));
    }
}
