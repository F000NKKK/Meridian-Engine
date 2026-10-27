//! Tests of the compressed-audio decoders.

use meridian_asset_domain::{AudioFormat, DecodeError, Decoder};

use crate::{AnyAudioDecoder, FlacDecoder, Mp3Decoder, OpusDecoder, VorbisDecoder};

/// A minimal single-page OGG stream: real page header (CRC left zero —
/// enough for signature detection, not for demuxing), one segment
/// carrying `payload`.
fn make_ogg_page(payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 255);
    let mut b = Vec::new();
    b.extend_from_slice(b"OggS");
    b.extend_from_slice(&[0u8; 22]); // version..CRC
    b.push(1); // one segment
    b.push(u8::try_from(payload.len()).unwrap()); // segment table
    b.extend_from_slice(payload);
    b
}

fn make_wav_header() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&36u32.to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b
}

/// The repo's real MP3 test asset, if present (skip otherwise — the
/// asset lives in `examples/`, not in this crate).
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
fn per_format_decoders_reject_wrong_signatures() {
    let wav = make_wav_header();
    assert!(matches!(
        Mp3Decoder.decode(&wav),
        Err(DecodeError::BadMagic { .. })
    ));
    assert!(matches!(
        VorbisDecoder.decode(&wav),
        Err(DecodeError::BadMagic { .. })
    ));
    assert!(matches!(
        OpusDecoder.decode(b"fLaC"),
        Err(DecodeError::BadMagic { .. })
    ));
    assert!(matches!(
        FlacDecoder.decode(b"ID3\x04"),
        Err(DecodeError::BadMagic { .. })
    ));
}

#[test]
fn malformed_bytes_error_instead_of_panicking() {
    // Valid signatures followed by garbage must produce an error,
    // never a panic or an empty success.
    let mut fake_mp3 = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
    fake_mp3.extend_from_slice(&[0xAB; 64]);
    assert!(Mp3Decoder.decode(&fake_mp3).is_err());

    let mut fake_opus = make_ogg_page(b"OpusHead\xFF\xFF");
    fake_opus.extend_from_slice(&[0xCD; 64]);
    assert!(OpusDecoder.decode(&fake_opus).is_err());

    assert!(FlacDecoder.decode(b"fLaC\xDE\xAD\xBE\xEF").is_err());
}

#[test]
fn any_audio_decoder_rejects_unknown_signatures() {
    assert!(matches!(
        AnyAudioDecoder.decode(b"garbage bytes here"),
        Err(DecodeError::Unsupported(_))
    ));
}

#[test]
fn any_audio_decoder_decodes_real_mp3_by_signature() {
    let Some(bytes) = demo_mp3() else { return };
    assert_eq!(AnyAudioDecoder.sniff(&bytes), Some(AudioFormat::Mp3));
    let audio = AnyAudioDecoder.decode(&bytes).unwrap();
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.channels, 2);
    assert!(
        !audio.samples.is_empty(),
        "decoded MP3 must produce samples"
    );
}
