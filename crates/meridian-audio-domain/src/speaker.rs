//! One speaker: its channel and horizontal direction.

use crate::Channel;

/// A speaker's horizontal position: degrees clockwise from straight ahead
/// (`0°`), matching the listener-local convention in the module doc
/// (positive = toward `+Z` = right). `None` means "never receives
/// directional panning" — real subwoofer (LFE) channels carry
/// bass-managed content derived from the other channels, not spatial
/// data; this crate doesn't implement bass management, so an LFE speaker
/// simply always gets `0.0` from [`SpeakerLayout::pan`](crate::SpeakerLayout::pan).
#[derive(Debug, Clone, Copy)]
pub struct Speaker {
    pub channel: Channel,
    pub azimuth_degrees: Option<f32>,
}
