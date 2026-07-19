//! Spatial audio built on the GAC: mixer, DSP graph, listener and emitter frames.

use meridian_gac_core::Motor3;

/// The listener's spatial frame for 3D audio.
#[derive(Debug, Clone, Copy, Default)]
pub struct Listener {
    pub frame: Motor3,
}

/// A sound source's spatial frame + playback state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Emitter {
    pub frame: Motor3,
}

/// Mixes active emitters down to the output stream.
#[derive(Debug, Default)]
pub struct Mixer;

/// A chain of DSP effect nodes.
#[derive(Debug, Default)]
pub struct DspGraph;
