//! A Symphonia format reader positioned on its default track.

use std::sync::Arc;

use meridian_asset_core::DecodeError;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::codec_registry::codec_registry;

/// An opened symphonia track: reader, decoder and the metadata the
/// container declares (any of which may be missing).
pub(crate) struct SymphoniaTrack {
    pub(crate) format: Box<dyn FormatReader>,
    pub(crate) decoder: Box<dyn AudioDecoder>,
    pub(crate) track_id: u32,
    pub(crate) total_frames: Option<u64>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) channels: Option<u16>,
}

/// Probes `bytes` with the shared codec registry pipeline and returns
/// the open reader/decoder pair plus the track's declared metadata.
pub(crate) fn open_symphonia(bytes: &Arc<[u8]>) -> Result<SymphoniaTrack, DecodeError> {
    let source = Box::new(std::io::Cursor::new(Arc::clone(bytes)));
    let stream = MediaSourceStream::new(source, Default::default());
    let format = symphonia::default::get_probe()
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
    let total_frames = track.num_frames;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(DecodeError::Unsupported("no audio codec parameters"))?;
    let params_rate = params.sample_rate;
    let params_channels = params.channels.as_ref().map(|c| c.count() as u16);

    let decoder = codec_registry()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    Ok(SymphoniaTrack {
        format,
        decoder,
        track_id,
        total_frames,
        sample_rate: params_rate,
        channels: params_channels,
    })
}
