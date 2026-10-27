//! Tests of this module.

use super::*;
use crate::Decoder;
use crate::audio::wav::WavDecoder;

fn make_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let fmt_body_len = 16u32;
    let data_len = data_bytes.len() as u32;
    let riff_len = 4 + (8 + fmt_body_len) + (8 + data_len);
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&riff_len.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&fmt_body_len.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&sample_rate.to_le_bytes());
    let block_align = channels * 2;
    b.extend_from_slice(&(sample_rate * block_align as u32).to_le_bytes());
    b.extend_from_slice(&block_align.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    b.extend_from_slice(&data_bytes);
    b
}

fn demo_mp3() -> Option<Vec<u8>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/assets/audio/demo-music.mp3"
    );
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            eprintln!("skipping: {path} not found");
            None
        }
    }
}

#[test]
fn wav_streaming_blocks_reassemble_the_eager_decode() {
    let samples: Vec<i16> = (0..1000).map(|i| (i * 7 % 1000) as i16).collect();
    let wav = make_wav(48_000, 2, &samples);

    let eager = WavDecoder.decode(&wav).unwrap();
    let mut stream = StreamingAudioDecoder::new(&wav, 64).unwrap();
    assert_eq!(stream.sample_rate(), 48_000);
    assert_eq!(stream.channels(), 2);

    let mut reassembled = Vec::new();
    while let Some(block) = stream.next_block().unwrap() {
        assert!(block.len() <= 64 * 2, "block exceeds configured size");
        reassembled.extend_from_slice(&block);
    }
    assert_eq!(reassembled, eager.samples);
}

#[test]
fn rewind_replays_the_first_block() {
    let samples: Vec<i16> = (0..500).map(|i| i as i16).collect();
    let wav = make_wav(44_100, 1, &samples);
    let mut stream = StreamingAudioDecoder::new(&wav, 100).unwrap();
    let first = stream.next_block().unwrap().unwrap();
    while stream.next_block().unwrap().is_some() {}
    stream.rewind().unwrap();
    assert_eq!(stream.next_block().unwrap().unwrap(), first);
}

#[test]
fn strategy_modes_pick_the_expected_path() {
    let samples = vec![0i16; 2000];
    let wav = make_wav(48_000, 2, &samples);

    // Tiny track, Auto with the default threshold -> eager.
    match open_audio(&wav, &DecodeStrategy::default()).unwrap() {
        AudioAsset::Decoded(audio) => assert_eq!(audio.samples.len(), 2000),
        AudioAsset::Streaming(_) => panic!("small track should decode eagerly in Auto"),
    }

    // Forced streaming wins regardless of size.
    let force = DecodeStrategy {
        mode: DecodeMode::ForceStreaming,
        ..Default::default()
    };
    assert!(matches!(
        open_audio(&wav, &force).unwrap(),
        AudioAsset::Streaming(_)
    ));

    // Auto with a threshold below the track size -> streaming.
    let tiny_threshold = DecodeStrategy {
        auto_threshold_bytes: 100,
        ..Default::default()
    };
    assert!(matches!(
        open_audio(&wav, &tiny_threshold).unwrap(),
        AudioAsset::Streaming(_)
    ));
}

#[test]
fn mp3_streaming_matches_eager_decode() {
    let Some(bytes) = demo_mp3() else { return };
    let eager = crate::AnyAudioDecoder.decode(&bytes).unwrap();

    let mut stream = StreamingAudioDecoder::new(&bytes, 4096).unwrap();
    assert_eq!(stream.sample_rate(), eager.sample_rate);
    assert_eq!(stream.channels(), eager.channels);

    let mut reassembled = Vec::new();
    while let Some(block) = stream.next_block().unwrap() {
        reassembled.extend_from_slice(&block);
    }
    assert_eq!(reassembled, eager.samples);

    // The demo track decodes to ~17 MB -> Auto must stream it.
    assert!(matches!(
        open_audio(&bytes, &DecodeStrategy::default()).unwrap(),
        AudioAsset::Streaming(_)
    ));
}

#[test]
fn iterator_face_yields_the_same_blocks() {
    let samples: Vec<i16> = (0..300).map(|i| i as i16).collect();
    let wav = make_wav(48_000, 1, &samples);
    let stream = StreamingAudioDecoder::new(&wav, 128).unwrap();
    let total: usize = stream.map(|block| block.unwrap().len()).sum();
    assert_eq!(total, 300);
}
