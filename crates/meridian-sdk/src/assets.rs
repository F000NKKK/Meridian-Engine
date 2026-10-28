//! Resource loading — decode + cache + register, the part of "put a
//! model on screen" that has nothing to do with scene composition
//! (procedural mesh generation lives in [`crate::scene`]; this module
//! is about turning a file *path* into a real GPU-registered handle,
//! once, no matter how many times an application asks for it).
//!
//! **Layering, explicitly:** `asset-domain` only decodes bytes into
//! CPU-side data (`ImageAsset`/`MeshAsset`) — it has no idea what a path,
//! a cache, or a GPU handle is (rule 4: asset-domain stays a decoder,
//! never a manager). `resource`'s generational handles are what
//! `MeshRegistry`/`TextureRegistry` (in `rendering-domain`) already build
//! identity on. This module is the missing middle layer an application
//! actually wants: "give me a path, get a handle back, and don't
//! re-decode/re-upload if I ask for the same path twice" — owned here,
//! not smeared across `asset-domain` (which shouldn't know about caching
//! or GPU handles) or `rendering-domain` (whose registries shouldn't know
//! about file paths or decoders).

use std::collections::{HashMap, VecDeque};
use std::path::Path;

use meridian_asset_domain::{Decoder, ImageAsset, ObjDecoder};
use meridian_asset_gltf::GltfDecoder;
use meridian_asset_symphonia::{AudioAsset, DecodeStrategy, StreamingAudioDecoder, open_audio};
use meridian_rendering_app::{MeshStore, TextureStore};
use meridian_rendering_domain::{
    MeshHandle, MeshRegistryError, MeshSource, TextureHandle, smooth_normals,
};

use crate::any_image_decoder::AnyImageDecoder;
pub use crate::model_loading::ModelPiece;
use crate::{model_files, model_loading};

/// Reads and decodes a real image asset file, identified by its magic
/// bytes (never its extension — the same rule
/// `asset-domain::AudioFormat::detect` follows for audio). `path` must
/// already be a path this process can open directly (absolute, or
/// relative to the current working directory) — this function doesn't
/// resolve it against any particular crate's `CARGO_MANIFEST_DIR`,
/// since this crate is a shared dependency of every application, not
/// the one that owns the asset directory; a caller with assets under
/// its own crate joins `env!("CARGO_MANIFEST_DIR")` itself before
/// calling this.
#[must_use]
pub fn load_image_asset(path: &str) -> ImageAsset {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("failed to read asset {path}: {e}"));
    AnyImageDecoder
        .decode(&bytes)
        .unwrap_or_else(|e| panic!("failed to decode asset {path}: {e}"))
}

/// One looping mono audio source's decoded samples, however
/// [`load_audio_track`] chose to get them — the two arms of
/// `asset-symphonia::AudioAsset`, exposed behind one
/// [`next_mono_chunk`](Self::next_mono_chunk) face so a caller never
/// needs to know which one it got. Downmixed to mono here (spatializers
/// like `audio-domain::BinauralRenderer` take one channel per source and
/// pan it themselves) — not cached by [`AssetCache`] like textures/
/// meshes, since a track carries its own playback cursor and two
/// listeners of the same file need two independent cursors, not one
/// shared handle.
#[derive(Debug)]
pub enum AudioTrack {
    Memory {
        mono: Vec<f32>,
        cursor: usize,
    },
    Streamed {
        decoder: StreamingAudioDecoder,
        channels: usize,
        queue: VecDeque<f32>,
    },
}

impl AudioTrack {
    /// Pulls the next `frames` mono samples, looping seamlessly when the
    /// source ends (rewinding a streamed decoder, wrapping a decoded
    /// buffer's cursor) — every track this loads is meant to loop
    /// indefinitely, e.g. background music, not a one-shot sound effect.
    pub fn next_mono_chunk(&mut self, frames: usize) -> Vec<f32> {
        let mut chunk = Vec::with_capacity(frames);
        match self {
            AudioTrack::Memory { mono, cursor } => {
                for _ in 0..frames {
                    chunk.push(mono[*cursor]);
                    *cursor = (*cursor + 1) % mono.len();
                }
            }
            AudioTrack::Streamed {
                decoder,
                channels,
                queue,
            } => {
                while chunk.len() < frames {
                    if let Some(sample) = queue.pop_front() {
                        chunk.push(sample);
                        continue;
                    }
                    if !refill_queue(decoder, *channels, queue) {
                        chunk.resize(frames, 0.0);
                        break;
                    }
                }
            }
        }
        chunk
    }
}

/// Decodes the next block into `queue` as mono samples, rewinding at the
/// end of the stream. `false` when the stream failed, so the caller pads
/// with silence.
fn refill_queue(
    decoder: &mut StreamingAudioDecoder,
    channels: usize,
    queue: &mut VecDeque<f32>,
) -> bool {
    match decoder.next_block() {
        Ok(Some(block)) => {
            for frame in block.chunks_exact(channels) {
                let sum: f32 = frame.iter().map(|&s| f32::from(s) / 32768.0).sum();
                queue.push_back(sum / channels as f32);
            }
            true
        }
        Ok(None) => decoder.rewind().is_ok(),
        Err(_) => false,
    }
}

/// Loads one audio file at `path` through `asset-symphonia::open_audio`'s
/// strategy-driven front door (`DecodeStrategy::default()` decodes short
/// tracks eagerly and streams long ones — this handles both arms
/// transparently) and downmixes it to mono. Returns the track plus its
/// native sample rate. A short in-memory track gets its loop seam
/// (last sample -> first) faded over ~10 ms so looping passes through
/// silence instead of clicking; a streamed track loops via
/// `StreamingAudioDecoder::rewind` instead, so no seam fade is needed
/// there.
pub fn load_audio_track(path: &str) -> Result<(AudioTrack, u32), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
    let asset = open_audio(&bytes, &DecodeStrategy::default())
        .map_err(|e| format!("failed to decode {path}: {e}"))?;
    let (sample_rate, channels) = (asset.sample_rate(), asset.channels().max(1) as usize);
    let track = match asset {
        AudioAsset::Decoded(audio) => {
            let mut mono: Vec<f32> = audio
                .samples
                .chunks_exact(channels)
                .map(|frame| {
                    frame.iter().map(|&s| f32::from(s) / 32768.0).sum::<f32>() / channels as f32
                })
                .collect();
            let fade = (sample_rate as usize / 100).min(mono.len() / 2);
            for i in 0..fade {
                let ramp = i as f32 / fade as f32;
                mono[i] *= ramp;
                let end = mono.len() - 1 - i;
                mono[end] *= ramp;
            }
            AudioTrack::Memory { mono, cursor: 0 }
        }
        AudioAsset::Streaming(decoder) => AudioTrack::Streamed {
            decoder,
            channels,
            queue: VecDeque::new(),
        },
    };
    Ok((track, sample_rate))
}

/// Path-keyed cache over texture/mesh loading — an application (or
/// [`crate::scene::GraphicsSession`], which owns one) calls
/// [`load_texture`](Self::load_texture)/[`load_mesh_obj`](Self::load_mesh_obj)
/// as many times as it wants for the same path (a texture shared by
/// several materials, a mesh instanced by several entities); only the
/// first call actually reads, decodes and uploads it.
#[derive(Debug, Default)]
pub struct AssetCache {
    textures: HashMap<String, TextureHandle>,
    meshes: HashMap<String, MeshHandle>,
    models: HashMap<String, Vec<ModelPiece>>,
}

impl AssetCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads `path` (see [`load_image_asset`]) as a GPU texture and
    /// returns its handle — cached by `path`: a second call with the
    /// same path returns the same handle without re-reading, decoding
    /// or uploading the file again.
    pub fn load_texture(&mut self, textures: &mut impl TextureStore, path: &str) -> TextureHandle {
        if let Some(&handle) = self.textures.get(path) {
            return handle;
        }
        let image = load_image_asset(path);
        let handle = textures.upload(&image);
        self.textures.insert(path.to_string(), handle);
        handle
    }

    /// Loads an OBJ model file at `path` through `asset-domain::ObjDecoder`
    /// (positions + triangle indices only — see that decoder's own
    /// module doc for the current "no `vt`/`vn` parsing" scope), derives
    /// smooth per-vertex normals from the triangle topology (this
    /// `rendering-domain`'s `smooth_normals`), and registers the
    /// result as a real [`MeshHandle`] — cached by `path`, same as
    /// [`load_texture`](Self::load_texture). UVs are `[0.0, 0.0]` for
    /// every vertex until the decoder itself gains `vt` support; a
    /// textured material still renders (uniformly sampling the
    /// texture's corner), it just won't be UV-mapped correctly yet —
    /// disclosed, not hidden.
    pub fn load_mesh_obj(
        &mut self,
        meshes: &mut impl MeshStore,
        path: &str,
    ) -> Result<MeshHandle, String> {
        if let Some(&handle) = self.meshes.get(path) {
            return Ok(handle);
        }
        let bytes = std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
        let mesh_data = ObjDecoder
            .decode(&bytes)
            .map_err(|e| format!("failed to decode {path}: {e}"))?;
        let normals = smooth_normals(&mesh_data.positions, &mesh_data.indices);
        let uvs = vec![[0.0, 0.0]; mesh_data.positions.len()];
        let source = MeshSource {
            positions: mesh_data.positions,
            normals,
            uvs,
            indices: mesh_data.indices,
        };
        let handle = meshes
            .add(source)
            .map_err(|e: MeshRegistryError| format!("{path}: {e}"))?;
        self.meshes.insert(path.to_string(), handle);
        Ok(handle)
    }

    /// Loads a `.glb` (or self-contained `.gltf`) model at `path` through
    /// `asset-gltf::GltfDecoder` and registers each part's mesh and its
    /// textures (ADR 030), cached by `path` like [`load_texture`](Self::load_texture).
    /// Skins and animations are ignored. A `.gltf` that names external
    /// buffers or images has them read from beside it.
    ///
    /// # Errors
    ///
    /// The file cannot be read, is not a supported glTF, or a part cannot be
    /// registered (see [`MeshRegistryError`]).
    pub fn load_model_gltf(
        &mut self,
        meshes: &mut impl MeshStore,
        textures: &mut impl TextureStore,
        path: &str,
    ) -> Result<Vec<ModelPiece>, String> {
        if let Some(pieces) = self.models.get(path) {
            return Ok(pieces.clone());
        }
        let bytes = std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
        let uris = GltfDecoder::required_uris(&bytes)
            .map_err(|e| format!("failed to decode {path}: {e}"))?;
        let base = Path::new(path).parent().unwrap_or_else(|| Path::new("."));
        let files = model_files::read_all(base, &uris).map_err(|e| format!("{path}: {e}"))?;
        let model = GltfDecoder::decode_with(&bytes, &files)
            .map_err(|e| format!("failed to decode {path}: {e}"))?;
        let pieces =
            model_loading::register(model, meshes, textures).map_err(|e| format!("{path}: {e}"))?;
        self.models.insert(path.to_string(), pieces.clone());
        Ok(pieces)
    }
}
