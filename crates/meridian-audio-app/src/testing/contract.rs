//! The behaviour every [`AudioSink`](crate::AudioSink) must have.

use crate::AudioSink;

/// Asserts the sink contract against fresh sinks from `make`.
///
/// A new sink has room for a block; zero samples always fit; fitting is
/// monotone; pushing a block of silence does not panic and leaves the sink
/// usable. Run by the fake in this crate and by each real adapter.
///
/// # Panics
///
/// Panics when `S` breaks the contract.
pub fn run_contract<S: AudioSink>(make: impl Fn() -> S) {
    let sink = make();
    assert!(sink.can_push(0), "zero samples always fit");
    assert!(sink.can_push(1), "a new sink has room");
    if sink.can_push(64) {
        assert!(sink.can_push(32), "room is monotone");
    }
    sink.push_interleaved(&[]);
    sink.push_interleaved(&[0.0; 64]);
    assert!(sink.can_push(0), "an empty request fits after a push");

    let second = make();
    assert!(second.can_push(1), "sinks are independent");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::RecordingSink;

    #[test]
    fn the_fake_passes_the_contract() {
        run_contract(|| RecordingSink::with_capacity(1024));
    }

    #[test]
    #[should_panic(expected = "a new sink has room")]
    fn a_sink_without_room_fails_the_contract() {
        run_contract(|| RecordingSink::with_capacity(0));
    }
}
