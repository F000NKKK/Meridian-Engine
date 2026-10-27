//! The codec set every symphonia-backed decoder in this crate uses.

use symphonia::core::codecs::registry::CodecRegistry;

/// Builds the codec registry for the compressed formats this crate
/// decodes: symphonia's own enabled codecs (MP3/Vorbis/FLAC) plus libopus
/// via the adapter — so Opus rides the exact same probe/demux/decode
/// pipeline instead of a hand-written OGG parse. A value built per
/// decoder, not a process-wide cache: registering a handful of codecs is
/// cheap next to decoding, and there is no global state to reason about.
pub(crate) fn codec_registry() -> CodecRegistry {
    let mut registry = CodecRegistry::new();
    symphonia::default::register_enabled_codecs(&mut registry);
    registry.register_audio_decoder::<symphonia_adapter_libopus::OpusDecoder>();
    registry
}
