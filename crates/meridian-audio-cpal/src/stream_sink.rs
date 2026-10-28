//! A thread-safe handle for pushing samples into an open stream.

use meridian_audio_app::AudioSink;

use crate::AudioStream;

/// The push side of an [`AudioStream`], movable to another thread.
///
/// The stream itself holds the platform's `cpal::Stream`, which is not
/// `Send` on every platform, so it stays on the thread that opened it; this
/// handle shares only the sample ring, so a runtime stage on a worker thread
/// can feed it. Pushing after the stream is dropped is a silent no-op.
#[derive(Debug, Clone)]
pub struct StreamSink {
    sample_tx: crossbeam_channel::Sender<f32>,
    capacity: usize,
}

impl AudioStream {
    /// A [`Send`] handle to this stream's sample ring.
    #[must_use]
    pub fn sink(&self) -> StreamSink {
        StreamSink {
            sample_tx: self.sample_tx.clone(),
            capacity: self.capacity,
        }
    }
}

impl AudioSink for StreamSink {
    fn push_interleaved(&self, samples: &[f32]) {
        for &sample in samples {
            if self.sample_tx.send(sample).is_err() {
                break;
            }
        }
    }

    fn can_push(&self, sample_count: usize) -> bool {
        self.capacity.saturating_sub(self.sample_tx.len()) >= sample_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sink_with_capacity(capacity: usize) -> (StreamSink, crossbeam_channel::Receiver<f32>) {
        let (sample_tx, rx) = crossbeam_channel::bounded(capacity);
        (
            StreamSink {
                sample_tx,
                capacity,
            },
            rx,
        )
    }

    #[test]
    fn the_sink_can_cross_threads() {
        fn assert_send<T: Send>() {}
        assert_send::<StreamSink>();
    }

    #[test]
    fn pushed_samples_reach_the_ring_in_order_and_use_up_room() {
        let (sink, rx) = sink_with_capacity(4);
        assert!(sink.can_push(4));

        sink.push_interleaved(&[0.1, 0.2, 0.3]);

        assert!(!sink.can_push(2));
        assert!(sink.can_push(1));
        assert_eq!(rx.try_iter().collect::<Vec<_>>(), vec![0.1, 0.2, 0.3]);
        assert!(sink.can_push(4));
    }

    #[test]
    fn pushing_after_the_stream_is_gone_is_a_no_op() {
        let (sink, rx) = sink_with_capacity(4);
        drop(rx);

        sink.push_interleaved(&[0.5; 3]);
    }
}
