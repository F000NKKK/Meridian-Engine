//! The stage that keeps an audio output topped up.

use meridian_audio_app::{AudioScene, AudioSink};

use crate::{Shared, Stage};

/// Most blocks a single run renders, so a sink that always reports room
/// cannot keep one frame busy forever.
const DEFAULT_MAX_BLOCKS: usize = 16;

/// Built-in stage that renders a shared [`AudioScene`] into an
/// [`AudioSink`]. Each run tops the sink up with whole blocks of
/// `block_frames` frames, mixed against the scene's current listener and
/// voice emitters, and stops at the first block the sink has no room for:
/// it never blocks the frame. The composition root picks the scene's
/// renderer and effects (speaker mixer, binaural, declicker) and the sink
/// (`meridian-audio-cpal`'s [`StreamSink`], or a fake).
///
/// [`StreamSink`]: https://docs.rs/meridian-audio-cpal
pub struct AudioStage {
    scene: Shared<AudioScene>,
    sink: Box<dyn AudioSink + Send>,
    block_frames: usize,
    max_blocks: usize,
    block: Vec<f32>,
}

impl AudioStage {
    /// A stage rendering `block_frames` frames per block from `scene` into
    /// `sink`. A shorter block reacts faster to a moving listener; a longer
    /// one costs less per sample.
    #[must_use]
    pub fn new(
        scene: Shared<AudioScene>,
        sink: impl AudioSink + Send + 'static,
        block_frames: usize,
    ) -> Self {
        let channels = scene.lock().channels();
        Self {
            scene,
            sink: Box::new(sink),
            block_frames,
            max_blocks: DEFAULT_MAX_BLOCKS,
            block: vec![0.0; block_frames * channels],
        }
    }

    /// Caps how many blocks one run may render (default 16).
    #[must_use]
    pub fn with_max_blocks_per_run(mut self, max_blocks: usize) -> Self {
        self.max_blocks = max_blocks;
        self
    }
}

impl std::fmt::Debug for AudioStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioStage")
            .field("block_frames", &self.block_frames)
            .field("max_blocks", &self.max_blocks)
            .finish_non_exhaustive()
    }
}

impl Stage for AudioStage {
    fn run(&mut self) {
        let mut scene = self.scene.lock();
        for _ in 0..self.max_blocks {
            if self.block.is_empty() || !self.sink.can_push(self.block.len()) {
                break;
            }
            scene.render_block(&mut self.block);
            self.sink.push_interleaved(&self.block);
        }
    }
}
