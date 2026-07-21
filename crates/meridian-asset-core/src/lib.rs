//! Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.
//!
//! This crate must never define an `AssetManager`, `ResourceManager`, or
//! `CacheManager` type — see `docs/dependency-rules.md` rule 4.
//!
//! Real decoders, not stubs. Images live in the [`image`] barrel module
//! (hand-rolled BMP, PNG via an external crate — see that module's own
//! doc for why); meshes get a hand-rolled minimal OBJ decoder directly
//! here (no format complex enough yet to need its own module). Audio
//! decoding lives in the [`audio`] barrel module — [`open_audio`] is its
//! thin, configuration-driven loading interface (full decode vs.
//! streaming per [`DecodeStrategy`]); compressed formats (MP3,
//! OGG/Vorbis, FLAC, OGG/Opus) use external codec crates per
//! docs/adr/013-compressed-audio-codecs.md and are identified by leading
//! magic bytes ([`AudioFormat::detect`]), never by file extension — the
//! same signature-not-extension rule [`image`]'s decoders follow. JPEG
//! and glTF remain future work on the same
//! when-a-concrete-asset-needs-it trigger.

pub mod audio;
pub mod image;

pub use audio::{
    AnyAudioDecoder, AudioAsset, AudioFormat, DecodeMode, DecodeStrategy, FlacDecoder, Mp3Decoder,
    OpusDecoder, StreamingAudioDecoder, VorbisDecoder, WavDecoder, open_audio,
};
pub use image::{BmpDecoder, PngDecoder};

/// Decodes raw file bytes into a CPU-side representation of `T`. Does not
/// decide where `T` lives afterward or when it's dropped.
pub trait Decoder<T> {
    type Error;

    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error>;
}

/// Decoded image data (RGBA8 pixels + dimensions), not yet uploaded to the GPU.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    /// Row-major, top-to-bottom, 4 bytes (R,G,B,A) per pixel.
    pub pixels: Vec<u8>,
}

/// Decoded mesh data (vertices/indices), not yet uploaded to the GPU.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// Decoded PCM audio data, not yet uploaded to an audio device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioData {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved 16-bit signed samples.
    pub samples: Vec<i16>,
}

/// Shader source, undecoded beyond a UTF-8 check — compiling it into a
/// `graphics-driver` shader module is that crate's job, not this one's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShaderSource {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    TooShort {
        needed: usize,
        got: usize,
    },
    BadMagic {
        expected: &'static str,
    },
    Unsupported(&'static str),
    Malformed(&'static str),
    /// An external codec library rejected the data — the message is the
    /// library's own (dynamic) error text.
    Codec(String),
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::TooShort { needed, got } => {
                write!(f, "buffer too short: needed {needed} bytes, got {got}")
            }
            DecodeError::BadMagic { expected } => write!(f, "bad magic bytes, expected {expected}"),
            DecodeError::Unsupported(what) => write!(f, "unsupported: {what}"),
            DecodeError::Malformed(what) => write!(f, "malformed: {what}"),
            DecodeError::Codec(what) => write!(f, "codec error: {what}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl meridian_foundation::EngineError for DecodeError {}

pub(crate) fn need(bytes: &[u8], len: usize) -> Result<(), DecodeError> {
    if bytes.len() < len {
        Err(DecodeError::TooShort {
            needed: len,
            got: bytes.len(),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub(crate) fn u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Decodes a minimal Wavefront OBJ: `v x y z` vertex lines and `f a b c`
/// triangle face lines (1-indexed; `a/b/c` texture/normal index suffixes
/// are recognized and ignored). No normals, UVs, materials, groups, or
/// non-triangular faces.
#[derive(Debug, Default)]
pub struct ObjDecoder;

impl Decoder<MeshData> for ObjDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<MeshData, DecodeError> {
        let text =
            core::str::from_utf8(bytes).map_err(|_| DecodeError::Malformed("not valid UTF-8"))?;
        let mut positions = Vec::new();
        let mut indices = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => {
                    let coords: Vec<f32> = fields
                        .map(|f| {
                            f.parse::<f32>().map_err(|_| {
                                DecodeError::Malformed("non-numeric vertex coordinate")
                            })
                        })
                        .collect::<Result<_, _>>()?;
                    if coords.len() != 3 {
                        return Err(DecodeError::Malformed(
                            "vertex line without exactly 3 coordinates",
                        ));
                    }
                    positions.push([coords[0], coords[1], coords[2]]);
                }
                Some("f") => {
                    let idx: Vec<u32> = fields
                        .map(|f| {
                            let vertex_index = f.split('/').next().unwrap_or(f);
                            vertex_index
                                .parse::<i64>()
                                .map_err(|_| DecodeError::Malformed("non-numeric face index"))
                        })
                        .collect::<Result<Vec<i64>, _>>()?
                        .into_iter()
                        .map(|one_indexed| (one_indexed - 1) as u32)
                        .collect();
                    if idx.len() != 3 {
                        return Err(DecodeError::Unsupported("non-triangular face"));
                    }
                    indices.extend(idx);
                }
                _ => {} // comments, normals, UVs, groups, etc. — ignored
            }
        }

        Ok(MeshData { positions, indices })
    }
}

/// Loads shader source as-is (UTF-8 text). Compiling it is
/// `graphics-driver`'s job.
#[derive(Debug, Default)]
pub struct ShaderSourceDecoder;

impl Decoder<ShaderSource> for ShaderSourceDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ShaderSource, DecodeError> {
        let source = core::str::from_utf8(bytes)
            .map_err(|_| DecodeError::Malformed("not valid UTF-8"))?
            .to_string();
        Ok(ShaderSource { source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_decodes_a_triangle() {
        let text = "# comment\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
        let mesh = ObjDecoder.decode(text.as_bytes()).unwrap();
        assert_eq!(
            mesh.positions,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        );
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn obj_ignores_texture_and_normal_index_suffixes() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1/1 2/2/2 3/3/3\n";
        let mesh = ObjDecoder.decode(text.as_bytes()).unwrap();
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn obj_rejects_faces_with_more_than_three_indices() {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nf 1 2 3 4\n";
        assert_eq!(
            ObjDecoder.decode(text.as_bytes()),
            Err(DecodeError::Unsupported("non-triangular face"))
        );
    }

    #[test]
    fn obj_rejects_faces_with_fewer_than_three_indices() {
        let text = "v 0 0 0\nv 1 0 0\nf 1 2\n";
        assert_eq!(
            ObjDecoder.decode(text.as_bytes()),
            Err(DecodeError::Unsupported("non-triangular face"))
        );
    }

    #[test]
    fn shader_source_decodes_utf8_text_verbatim() {
        let text = "#version 450\nvoid main() {}\n";
        let shader = ShaderSourceDecoder.decode(text.as_bytes()).unwrap();
        assert_eq!(shader.source, text);
    }

    #[test]
    fn shader_source_rejects_invalid_utf8() {
        let bytes = [0xff, 0xfe, 0xfd];
        assert!(matches!(
            ShaderSourceDecoder.decode(&bytes),
            Err(DecodeError::Malformed(_))
        ));
    }
}
