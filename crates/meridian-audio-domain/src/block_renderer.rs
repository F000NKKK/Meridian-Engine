//! The port between a set of voices and interleaved output samples.

use crate::{BinauralRenderer, Emitter, Listener, Mixer};

/// Renders blocks of mono sources, as heard by a listener, into interleaved
/// output. The composition root picks the implementation: the stateless
/// speaker-gain [`Mixer`] or the stateful headphone [`BinauralRenderer`].
pub trait BlockRenderer: Send {
    /// Interleaved channels per frame in the rendered output.
    fn channels(&self) -> usize;

    /// Overwrites `out` (`out.len() / channels()` frames) with the mix of
    /// `sources`; a source shorter than the block is padded with silence.
    fn render(&mut self, listener: &Listener, sources: &[(Emitter, &[f32])], out: &mut [f32]);
}

impl BlockRenderer for Mixer {
    fn channels(&self) -> usize {
        self.layout.speakers.len()
    }

    fn render(&mut self, listener: &Listener, sources: &[(Emitter, &[f32])], out: &mut [f32]) {
        self.render_interleaved_into(listener, sources, out);
    }
}

impl BlockRenderer for BinauralRenderer {
    fn channels(&self) -> usize {
        2
    }

    fn render(&mut self, listener: &Listener, sources: &[(Emitter, &[f32])], out: &mut [f32]) {
        self.render_into(listener, sources, out);
    }
}
