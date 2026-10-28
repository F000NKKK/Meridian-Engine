//! Per-shape music tracks mixed to the speakers.

use meridian_examples::paths::asset_path;
use meridian_sdk::{
    AcousticMedium, AudioScene, AudioStage, AudioStream, AudioTrack, BinauralRenderer, Declicker,
    Emitter, Listener, Motor3, ResampledSource, Runtime, SampleSource, Shared, SpeakerLayout, Vec3,
    Voice, VoiceId, load_audio_track, open_output, parallel_runtime,
};

use crate::ShapeSpec;

/// 10 ms of audio per mixed block — the listener pose is re-sampled at
/// 100 Hz, well past the point where parameter updates read as steps.
const CHUNK_SECONDS: f32 = 0.01;

/// ~80 ms ring: generous headroom so a dropped render frame never
/// drains it (see `audio-cpal`'s anti-click fade for the fallback if
/// it ever does) — see the former `music_sphere` example's identical
/// note, now superseded by this one.
const RING_SECONDS: f32 = 0.08;

/// Loads every [`ShapeSpec`](crate::ShapeSpec)'s track through `meridian_sdk::load_audio_track`
/// (decode/downmix/loop-seam-fade is generic asset-loading logic, owned
/// by the SDK — see `meridian_sdk::assets`'s module doc; only "which
/// files, what to do if one fails" is this example's own policy). The
/// stream plays at the first loaded track's rate; a track recorded at
/// another rate is resampled to it. A shape whose file fails to load or
/// decode plays silence (an all-zero `AudioTrack::Memory`) rather than
/// aborting the whole example — one bad asset shouldn't take the other
/// two down with it.
fn load_music_tracks(shapes: &[ShapeSpec]) -> (Vec<Box<dyn SampleSource>>, u32) {
    let loaded: Vec<(AudioTrack, Option<u32>)> = shapes
        .iter()
        .map(
            |shape| match load_audio_track(&asset_path(&shape.audio_file)) {
                Ok((track, rate)) => {
                    println!("{}: playing {} ({rate} Hz)", shape.name, shape.audio_file);
                    (track, Some(rate))
                }
                Err(err) => {
                    meridian_sdk::log_warn!("{}: running silent ({err})", shape.name);
                    let silence = AudioTrack::Memory {
                        mono: vec![0.0; 48_000],
                        cursor: 0,
                    };
                    (silence, None)
                }
            },
        )
        .collect();
    let sample_rate = loaded.iter().find_map(|(_, rate)| *rate).unwrap_or(48_000);
    let tracks = loaded
        .into_iter()
        .map(|(track, rate)| match rate {
            Some(rate) if rate != sample_rate => {
                Box::new(ResampledSource::new(track, rate, sample_rate)) as Box<dyn SampleSource>
            }
            _ => Box::new(track),
        })
        .collect();
    (tracks, sample_rate)
}

/// All tracks as looping voices of one [`AudioScene`], spatialized by one
/// shared `BinauralRenderer` and pushed into one `AudioStream` by an
/// [`AudioStage`] — see the module doc for why one stream suffices. The
/// example owns only what is its own: which voice follows which shape and
/// where the listener is.
pub(crate) struct MusicRig {
    /// Keeps the device stream alive; the stage feeds it through a `StreamSink`.
    _output: AudioStream,
    scene: Shared<AudioScene>,
    voices: Vec<VoiceId>,
    runtime: Runtime,
}

impl MusicRig {
    pub(crate) async fn load(shapes: &[ShapeSpec]) -> Result<Self, String> {
        let (tracks, sample_rate) = load_music_tracks(shapes);
        let ring_frames = (sample_rate as f32 * RING_SECONDS) as u32;
        let output = open_output(
            &SpeakerLayout::stereo_headphones(),
            sample_rate,
            Some(ring_frames),
        )
        .await
        .map_err(|e| e.to_string())?;

        let mut scene = AudioScene::new(
            BinauralRenderer::new(sample_rate).with_medium(AcousticMedium::air_sea_level()),
        )
        .with_post_effect(Declicker::new(2));
        let voices = tracks
            .into_iter()
            .map(|track| scene.add_voice(Voice::new(track, Emitter::default())))
            .collect();
        let scene = Shared::new(scene);

        let chunk_frames = (sample_rate as f32 * CHUNK_SECONDS) as usize;
        let mut runtime = parallel_runtime();
        runtime.add_stage(
            "audio",
            &[],
            AudioStage::new(scene.clone(), output.sink(), chunk_frames),
        );
        Ok(Self {
            _output: output,
            scene,
            voices,
            runtime,
        })
    }

    /// Points the listener and every voice at their current poses, then
    /// lets the audio stage top the output ring up. Never blocks: the stage
    /// pushes only while the stream reports room for a whole block.
    pub(crate) fn refill(&mut self, listener: &Listener, positions: &[Vec3]) {
        {
            let mut scene = self.scene.lock();
            scene.listener = *listener;
            for (&voice, &position) in self.voices.iter().zip(positions) {
                scene.set_emitter(
                    voice,
                    Emitter {
                        frame: Motor3::translation(position),
                    },
                );
            }
        }
        self.runtime.tick();
    }
}
