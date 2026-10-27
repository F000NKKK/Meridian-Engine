//! Fake and contract suite for the [`AudioSink`](crate::AudioSink) port (Craftbook 8.4).

mod contract;
mod recording_sink;

pub use contract::run_contract;
pub use recording_sink::RecordingSink;
