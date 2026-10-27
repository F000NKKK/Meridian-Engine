# meridian-asset-symphonia

Compressed-audio adapter for `meridian-asset-core`: MP3, OGG/Vorbis, FLAC and OGG/Opus on `symphonia` (pure Rust) and the reference `libopus`.

**Layer and context:** infrastructure · assets

- **For:** `Mp3Decoder`, `VorbisDecoder`, `FlacDecoder` and `OpusDecoder`, each a `Decoder<AudioClip>`; `AnyAudioDecoder`, which sniffs the format by magic bytes (WAV included); and incremental decoding: `StreamingAudioDecoder`, `open_audio` and `DecodeStrategy` (ADR 013, ADR 014).
- **Not for:** the asset types, `Decoder` port, WAV and format detection: those are `meridian-asset-core`. Reading files and deciding who owns the decoded samples is the caller's job.

## Usage

```rust
use meridian_asset_core::Decoder;
use meridian_asset_symphonia::AnyAudioDecoder;

// Garbage bytes are rejected with a typed error, never a panic.
assert!(AnyAudioDecoder.decode(b"not audio").is_err());
```

## Depends on / used by

- Depends on: [`meridian-asset-core`](../meridian-asset-core)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
