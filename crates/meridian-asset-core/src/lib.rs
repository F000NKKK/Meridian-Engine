//! Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.
//!
//! This crate must never define an `AssetManager`, `ResourceManager`, or
//! `CacheManager` type — see `docs/dependency-rules.md` rule 4.
//!
//! Real decoders, not stubs. Hand-rolled for formats simple enough to
//! not need an external crate: uncompressed BMP for images, PCM WAV for
//! audio, plain-text OBJ (positions + triangle indices only, no
//! normals/UVs/materials) for meshes. Audio decoding lives in the
//! [`audio`] barrel module — [`open_audio`] is its thin, configuration-
//! driven loading interface (full decode vs. streaming per
//! [`DecodeStrategy`]); compressed formats (MP3, OGG/Vorbis, FLAC,
//! OGG/Opus) use external codec crates per
//! docs/adr/013-compressed-audio-codecs.md and are identified by leading
//! magic bytes ([`AudioFormat::detect`]), never by file extension.
//! PNG decoding uses the `png` crate — see [`PngDecoder`] and
//! docs/adr/013-compressed-audio-codecs.md's DEFLATE-is-too-big-to-hand-roll
//! reasoning, which applies identically here. JPEG and glTF remain
//! future work on the same when-a-concrete-asset-needs-it trigger.

pub mod audio;

pub use audio::{
    AnyAudioDecoder, AudioAsset, AudioFormat, DecodeMode, DecodeStrategy, FlacDecoder, Mp3Decoder,
    OpusDecoder, StreamingAudioDecoder, VorbisDecoder, WavDecoder, open_audio,
};

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

fn i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Decodes uncompressed 24-bit or 32-bit BMP (`BITMAPINFOHEADER`, `BI_RGB`
/// only — no RLE compression, no palette/indexed color).
#[derive(Debug, Default)]
pub struct BmpDecoder;

impl Decoder<ImageData> for BmpDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageData, DecodeError> {
        need(bytes, 54)?; // 14-byte file header + 40-byte BITMAPINFOHEADER
        if &bytes[0..2] != b"BM" {
            return Err(DecodeError::BadMagic { expected: "BM" });
        }
        let pixel_offset = u32_le(bytes, 10) as usize;
        let dib_header_size = u32_le(bytes, 14);
        if dib_header_size < 40 {
            return Err(DecodeError::Unsupported(
                "DIB header smaller than BITMAPINFOHEADER",
            ));
        }
        let width = i32_le(bytes, 18);
        let height_raw = i32_le(bytes, 22);
        let bits_per_pixel = u16_le(bytes, 28);
        let compression = u32_le(bytes, 30);

        if compression != 0 {
            return Err(DecodeError::Unsupported(
                "BMP compression other than BI_RGB",
            ));
        }
        if width <= 0 {
            return Err(DecodeError::Malformed("non-positive width"));
        }
        let (height, top_down) = if height_raw < 0 {
            (-height_raw, true)
        } else {
            (height_raw, false)
        };
        if height <= 0 {
            return Err(DecodeError::Malformed("zero height"));
        }
        let (width, height) = (width as u32, height as u32);
        let src_bytes_per_pixel = match bits_per_pixel {
            24 => 3,
            32 => 4,
            _ => return Err(DecodeError::Unsupported("BMP bit depth other than 24/32")),
        };

        let row_stride = (width as usize * src_bytes_per_pixel).div_ceil(4) * 4; // rows padded to 4 bytes
        need(bytes, pixel_offset + row_stride * height as usize)?;

        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        for y in 0..height as usize {
            // BMP rows are bottom-to-top unless the height field was negative.
            let src_row = if top_down { y } else { height as usize - 1 - y };
            let row_start = pixel_offset + src_row * row_stride;
            for x in 0..width as usize {
                let src = row_start + x * src_bytes_per_pixel;
                let dst = (y * width as usize + x) * 4;
                // BMP stores B,G,R(,A); we produce R,G,B,A.
                pixels[dst] = bytes[src + 2];
                pixels[dst + 1] = bytes[src + 1];
                pixels[dst + 2] = bytes[src];
                pixels[dst + 3] = if src_bytes_per_pixel == 4 {
                    bytes[src + 3]
                } else {
                    255
                };
            }
        }

        Ok(ImageData {
            width,
            height,
            pixels,
        })
    }
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

    /// Builds a minimal uncompressed 24-bit BMP: `width`x`height`, every
    /// pixel the given (r,g,b).
    fn make_bmp(width: u32, height: u32, rgb: (u8, u8, u8)) -> Vec<u8> {
        let row_stride = (width as usize * 3).div_ceil(4) * 4;
        let pixel_data_len = row_stride * height as usize;
        let file_size = 54 + pixel_data_len;

        let mut b = Vec::with_capacity(file_size);
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(file_size as u32).to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0]); // reserved
        b.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset
        b.extend_from_slice(&40u32.to_le_bytes()); // DIB header size
        b.extend_from_slice(&(width as i32).to_le_bytes());
        b.extend_from_slice(&(height as i32).to_le_bytes()); // positive -> bottom-up
        b.extend_from_slice(&1u16.to_le_bytes()); // planes
        b.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
        b.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        b.extend_from_slice(&(pixel_data_len as u32).to_le_bytes());
        b.extend_from_slice(&2835i32.to_le_bytes()); // x pixels/meter
        b.extend_from_slice(&2835i32.to_le_bytes()); // y pixels/meter
        b.extend_from_slice(&0u32.to_le_bytes()); // colors used
        b.extend_from_slice(&0u32.to_le_bytes()); // important colors

        for _ in 0..height {
            for _ in 0..width {
                b.push(rgb.2); // B
                b.push(rgb.1); // G
                b.push(rgb.0); // R
            }
            b.extend(std::iter::repeat_n(0u8, row_stride - width as usize * 3)); // row padding
        }
        b
    }

    #[test]
    fn bmp_decodes_solid_color_and_dimensions() {
        let bytes = make_bmp(3, 2, (10, 20, 30));
        let image = BmpDecoder.decode(&bytes).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.pixels.len(), 3 * 2 * 4);
        for px in image.pixels.chunks_exact(4) {
            assert_eq!(px, &[10, 20, 30, 255]);
        }
    }

    #[test]
    fn bmp_row_padding_and_bottom_up_order_are_handled() {
        // width=1 forces row padding (3 bytes -> padded to 4); make row 0
        // (bottom, stored first) a different color from the top row to
        // verify vertical flip.
        let mut b = make_bmp(1, 2, (0, 0, 0));
        // Overwrite bottom row (first in file) to red, top row (second in
        // file) stays black, so decoded row 0 (top of image) must be black
        // and row 1 (bottom) must be red.
        let pixel_start = 54;
        b[pixel_start] = 0; // B
        b[pixel_start + 1] = 0; // G
        b[pixel_start + 2] = 255; // R

        let image = BmpDecoder.decode(&b).unwrap();
        let top_row_pixel = &image.pixels[0..4];
        let bottom_row_pixel = &image.pixels[4..8];
        assert_eq!(
            top_row_pixel,
            &[0, 0, 0, 255],
            "top row must come from the last row in the file"
        );
        assert_eq!(
            bottom_row_pixel,
            &[255, 0, 0, 255],
            "bottom row must come from the first row in the file"
        );
    }

    #[test]
    fn bmp_rejects_bad_magic() {
        let mut bytes = make_bmp(1, 1, (0, 0, 0));
        bytes[0] = b'X';
        assert_eq!(
            BmpDecoder.decode(&bytes),
            Err(DecodeError::BadMagic { expected: "BM" })
        );
    }

    #[test]
    fn bmp_rejects_truncated_buffer() {
        assert!(matches!(
            BmpDecoder.decode(&[0u8; 10]),
            Err(DecodeError::TooShort { .. })
        ));
    }

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
