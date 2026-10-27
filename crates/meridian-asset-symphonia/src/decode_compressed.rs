//! Whole-track decoding of the compressed formats.

use meridian_asset_core::{AudioClip, AudioFormat, DecodeError};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::codec_registry::codec_registry;

/// Decodes any compressed format `symphonia` handles here (MP3,
/// OGG/Vorbis, FLAC): probe the container, decode the default audio
/// track to completion, convert to interleaved `i16`.
pub(crate) fn decode_compressed(bytes: &[u8]) -> Result<AudioClip, DecodeError> {
    let source = Box::new(std::io::Cursor::new(bytes.to_vec()));
    let stream = MediaSourceStream::new(source, Default::default());

    let mut format = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(DecodeError::Unsupported("no audio track in container"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(DecodeError::Unsupported("no audio codec parameters"))?;

    let mut decoder = codec_registry()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut samples: Vec<i16> = Vec::new();
    let mut block: Vec<i16> = Vec::new();

    while let Some(packet) = format
        .next_packet()
        .map_err(|e| DecodeError::Codec(e.to_string()))?
    {
        if packet.track_id != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| DecodeError::Codec(e.to_string()))?;
        let spec = decoded.spec();
        sample_rate = spec.rate();
        channels = spec.channels().count() as u16;
        decoded.copy_to_vec_interleaved::<i16>(&mut block);
        samples.extend_from_slice(&block);
    }

    if samples.is_empty() {
        return Err(DecodeError::Malformed("no decodable audio packets"));
    }
    Ok(AudioClip {
        sample_rate,
        channels,
        samples,
    })
}

/// Guards that `bytes` really carries `expected`'s signature before
/// handing them to the codec — so e.g. [`Mp3Decoder`] on OGG bytes fails
/// with `BadMagic`, not a codec-internal error.
pub(crate) fn expect_format(bytes: &[u8], expected: AudioFormat) -> Result<(), DecodeError> {
    match AudioFormat::detect(bytes) {
        Some(found) if found == expected => Ok(()),
        _ => Err(DecodeError::BadMagic {
            expected: match expected {
                AudioFormat::Wav => "RIFF/WAVE",
                AudioFormat::Mp3 => "ID3 or MPEG frame sync",
                AudioFormat::OggVorbis => "OggS + \\x01vorbis",
                AudioFormat::OggOpus => "OggS + OpusHead",
                AudioFormat::Flac => "fLaC",
            },
        }),
    }
}
