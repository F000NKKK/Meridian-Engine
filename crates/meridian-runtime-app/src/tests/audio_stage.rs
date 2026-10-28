//! Tests of the audio stage.

use std::sync::Arc;

use meridian_audio_app::AudioScene;
use meridian_audio_app::testing::RecordingSink;
use meridian_audio_domain::{Emitter, MemorySource, Mixer, Repeat, SpeakerLayout, Voice};

use crate::testing::InlineExecutor;
use crate::*;

fn scene_with_one_voice() -> Shared<AudioScene> {
    let mut scene = AudioScene::new(Mixer::new(SpeakerLayout::mono()));
    scene.add_voice(Voice::new(
        MemorySource::new(vec![0.5; 16], Repeat::Loop),
        Emitter::default(),
    ));
    Shared::new(scene)
}

/// Shares a [`RecordingSink`] between the stage and the test.
struct SharedSink(Arc<RecordingSink>);

impl meridian_audio_app::AudioSink for SharedSink {
    fn push_interleaved(&self, samples: &[f32]) {
        self.0.push_interleaved(samples);
    }

    fn can_push(&self, sample_count: usize) -> bool {
        self.0.can_push(sample_count)
    }
}

#[test]
fn a_tick_fills_the_sink_with_whole_blocks_and_stops_when_it_is_full() {
    let sink = Arc::new(RecordingSink::with_capacity(10));
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    runtime.add_stage(
        "audio",
        &[],
        AudioStage::new(scene_with_one_voice(), SharedSink(sink.clone()), 4),
    );

    runtime.tick();

    // Room for 10 samples: two whole 4-sample blocks fit, a third does not.
    assert_eq!(sink.recorded().len(), 8);
    runtime.tick();
    assert_eq!(sink.recorded().len(), 8, "a full sink is left alone");
    sink.drain();
    runtime.tick();
    assert_eq!(sink.recorded().len(), 8);
}

#[test]
fn the_per_run_cap_bounds_a_sink_that_always_has_room() {
    let sink = Arc::new(RecordingSink::with_capacity(usize::MAX));
    let mut stage = AudioStage::new(scene_with_one_voice(), SharedSink(sink.clone()), 4)
        .with_max_blocks_per_run(3);

    stage.run();

    assert_eq!(sink.recorded().len(), 12);
}

#[test]
fn blocks_continue_the_voice_where_the_last_run_stopped() {
    let sink = Arc::new(RecordingSink::with_capacity(usize::MAX));
    let mut scene = AudioScene::new(Mixer::new(SpeakerLayout::mono()));
    scene.add_voice(Voice::new(
        MemorySource::new(vec![0.1, 0.2, 0.3, 0.4], Repeat::Loop),
        Emitter::default(),
    ));
    let mut stage =
        AudioStage::new(Shared::new(scene), SharedSink(sink.clone()), 2).with_max_blocks_per_run(1);

    stage.run();
    stage.run();

    let heard = sink.recorded();
    assert_eq!(heard.len(), 4);
    assert!(
        heard[2] > heard[1],
        "second block continues the ramp: {heard:?}"
    );
}
