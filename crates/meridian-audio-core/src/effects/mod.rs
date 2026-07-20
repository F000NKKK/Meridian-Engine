//! Audio effects, one per file (barrel module): reusable DSP building
//! blocks ([`DspNode`] implementations chained by [`DspGraph`]) and the
//! stateful [`BinauralRenderer`] headphone spatializer. New effects get
//! their own file here and a re-export below.

mod binaural;
mod dsp_graph;
mod gain;
mod low_pass;
mod medium;

pub use binaural::{BinauralConfig, BinauralRenderer};
pub use dsp_graph::DspGraph;
pub use gain::Gain;
pub use low_pass::LowPassFilter;
pub use medium::AcousticMedium;

/// A single DSP effect: mutates a buffer of samples in place.
pub trait DspNode {
    fn process(&mut self, samples: &mut [f32]);
}
