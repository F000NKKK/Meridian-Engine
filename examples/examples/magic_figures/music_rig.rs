//! Per-shape music tracks mixed to the speakers.

use meridian_examples::paths::asset_path;
use meridian_sdk::{
    AcousticMedium, AudioSink, AudioStream, AudioTrack, BinauralRenderer, Declicker, DspNode,
    Emitter, Listener, Motor3, SpeakerLayout, Vec3, load_audio_track, open_output,
};

use crate::ShapeSpec;

/// 10 ms of audio per mixed block — the listener pose is re-sampled at
/// 100 Hz, well past the point where parameter updates read as steps.
const CHUNK_SECONDS: f32 = 0.01;

/// ~80 ms ring: generous headroom so a dropped render frame never
/// drains it (see `audio-driver`'s anti-click fade for the fallback if
/// it ever does) — see the former `music_sphere` example's identical
/// note, now superseded by this one.
const RING_SECONDS: f32 = 0.08;

/// Loads every [`ShapeSpec`](crate::ShapeSpec)'s track through `meridian_sdk::load_audio_track`
/// (decode/downmix/loop-seam-fade is generic asset-loading logic, owned
/// by the SDK — see `meridian_sdk::assets`'s module doc; only "which
/// files, what to do if one fails" is this example's own policy). A
/// shape whose file fails to load or decode plays silence (an all-zero
/// `AudioTrack::Memory`) rather than aborting the whole example — one
/// bad asset shouldn't take the other two down with it.
fn load_music_tracks(shapes: &[ShapeSpec]) -> (Vec<AudioTrack>, u32) {
    let mut sample_rate = 48_000;
    let tracks = shapes
        .iter()
        .map(
            |shape| match load_audio_track(&asset_path(&shape.audio_file)) {
                Ok((track, rate)) => {
                    sample_rate = rate;
                    println!("{}: playing {} ({rate} Hz)", shape.name, shape.audio_file);
                    track
                }
                Err(err) => {
                    meridian_sdk::log_warn!("{}: running silent ({err})", shape.name);
                    AudioTrack::Memory {
                        mono: vec![0.0; 48_000],
                        cursor: 0,
                    }
                }
            },
        )
        .collect();
    (tracks, sample_rate)
}

/// All tracks, spatialized in one shared `BinauralRenderer` and pushed
/// into one `AudioStream` — see the module doc for why one stream
/// suffices, and for why this is a hand-assembled pipeline rather than
/// `meridian_sdk::AudioSubsystem::mix`.
pub(crate) struct MusicRig {
    output: AudioStream,
    renderer: BinauralRenderer,
    declicker: Declicker,
    tracks: Vec<AudioTrack>,
    chunk_frames: usize,
}

impl MusicRig {
    pub(crate) async fn load(shapes: &[ShapeSpec]) -> Result<Self, String> {
        let (tracks, sample_rate) = load_music_tracks(shapes);
        let renderer =
            BinauralRenderer::new(sample_rate).with_medium(AcousticMedium::air_sea_level());
        let ring_frames = (sample_rate as f32 * RING_SECONDS) as u32;
        let output = open_output(
            &SpeakerLayout::stereo_headphones(),
            sample_rate,
            Some(ring_frames),
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(Self {
            output,
            renderer,
            declicker: Declicker::new(2),
            tracks,
            chunk_frames: (sample_rate as f32 * CHUNK_SECONDS) as usize,
        })
    }

    /// Tops the output ring buffer up with blocks mixed against
    /// `listener`'s current pose and each shape's current
    /// `positions[i]`. Never blocks: pushes only while the stream
    /// reports room for a whole block.
    pub(crate) fn refill(&mut self, listener: &Listener, positions: &[Vec3]) {
        while self.output.can_push(self.chunk_frames * 2) {
            let chunks: Vec<Vec<f32>> = self
                .tracks
                .iter_mut()
                .map(|track| track.next_mono_chunk(self.chunk_frames))
                .collect();
            let sources: Vec<(Emitter, &[f32])> = positions
                .iter()
                .zip(&chunks)
                .map(|(&position, chunk)| {
                    (
                        Emitter {
                            frame: Motor3::translation(position),
                        },
                        chunk.as_slice(),
                    )
                })
                .collect();
            let mut interleaved = self.renderer.render(listener, &sources, self.chunk_frames);
            self.declicker.process(&mut interleaved);
            self.output.push_interleaved(&interleaved);
        }
    }
}
