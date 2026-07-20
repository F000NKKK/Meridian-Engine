//! Roadmap milestone: compressed audio assets decoded by *signature* and
//! played through the real output path. Scans `examples/assets/audio/`,
//! sniffs each file's format from its leading magic bytes
//! (`asset-core::AnyAudioDecoder` — the file extension is never
//! consulted), decodes to PCM, and plays the first few seconds of each
//! through `audio-core::AudioOutput`. Drop `.mp3`/`.opus`/`.ogg`/
//! `.flac`/`.wav` files into that directory (any name, any extension —
//! even a wrong one) and they play; see
//! docs/adr/013-compressed-audio-codecs.md.
//!
//! Run with:
//!   ./build.sh run audio_formats

use meridian_asset_core::{AnyAudioDecoder, AudioData, Decoder};
use meridian_audio_core::{AudioOutput, SpeakerLayout};

const PLAY_SECONDS: f32 = 5.0;

fn layout_for(channels: u16) -> Option<SpeakerLayout> {
    match channels {
        1 => Some(SpeakerLayout::mono()),
        2 => Some(SpeakerLayout::stereo_speakers()),
        _ => None,
    }
}

async fn play(audio: &AudioData) -> Result<(), String> {
    let layout = layout_for(audio.channels)
        .ok_or_else(|| format!("{} channels: no playback layout", audio.channels))?;
    let output = AudioOutput::open(&layout, audio.sample_rate)
        .await
        .map_err(|e| e.to_string())?;

    let total = ((PLAY_SECONDS * audio.sample_rate as f32) as usize
        * audio.channels as usize)
        .min(audio.samples.len());
    // Push in ~50 ms chunks; push_interleaved's blocking-on-full paces
    // the loop at the hardware's real playback rate.
    let chunk = (audio.sample_rate as usize / 20) * audio.channels as usize;
    for samples in audio.samples[..total].chunks(chunk) {
        let block: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();
        output.push_interleaved(&block);
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/audio");
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(entries) => entries.flatten().map(|e| e.path()).collect(),
        Err(err) => {
            println!("skipping: cannot read {dir} ({err})");
            return;
        }
    };
    entries.sort();

    if entries.is_empty() {
        println!("skipping: no files in {dir}");
        return;
    }

    for path in entries {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                println!("  {name}: unreadable ({err})");
                continue;
            }
        };

        match AnyAudioDecoder.sniff(&bytes) {
            Some(format) => {
                print!("  {name}: signature says {format:?}");
                match AnyAudioDecoder.decode(&bytes) {
                    Ok(audio) => {
                        let seconds = audio.samples.len() as f32
                            / (audio.sample_rate as f32 * audio.channels as f32);
                        println!(
                            " -> {} Hz, {} ch, {seconds:.1}s; playing first {:.0}s ...",
                            audio.sample_rate,
                            audio.channels,
                            PLAY_SECONDS.min(seconds),
                        );
                        if let Err(err) = play(&audio).await {
                            println!("    (not played: {err})");
                        }
                    }
                    Err(err) => println!(" but decoding failed: {err}"),
                }
            }
            None => println!("  {name}: unrecognized signature, skipped"),
        }
    }

    println!("done.");
}
