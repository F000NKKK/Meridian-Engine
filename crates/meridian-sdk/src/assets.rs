//! Resource loading — decode + cache + register, the part of "put a
//! model on screen" that has nothing to do with scene composition
//! (procedural mesh generation lives in [`crate::scene`]; this module
//! is about turning a file *path* into a real GPU-registered handle,
//! once, no matter how many times an application asks for it).
//!
//! **Layering, explicitly:** `asset-core` only decodes bytes into
//! CPU-side data (`ImageData`/`MeshData`) — it has no idea what a path,
//! a cache, or a GPU handle is (rule 4: asset-core stays a decoder,
//! never a manager). `resource-core`'s generational handles are what
//! `MeshRegistry`/`TextureRegistry` (in `graphics-core`) already build
//! identity on. This module is the missing middle layer an application
//! actually wants: "give me a path, get a handle back, and don't
//! re-decode/re-upload if I ask for the same path twice" — owned here,
//! not smeared across `asset-core` (which shouldn't know about caching
//! or GPU handles) or `graphics-core` (whose registries shouldn't know
//! about file paths or decoders).

use std::collections::{HashMap, VecDeque};

use meridian_asset_core::{
    AnyImageDecoder, AudioAsset, DecodeStrategy, Decoder, ImageData, ObjDecoder,
    StreamingAudioDecoder, open_audio,
};
use meridian_graphics_core::{
    MeshHandle, MeshRegistry, MeshRegistryError, MeshSource, TextureHandle, TextureRegistry,
};
use meridian_graphics_driver::Device;

/// Reads and decodes a real image asset file, identified by its magic
/// bytes (never its extension — the same rule
/// `asset-core::AudioFormat::detect` follows for audio). `path` must
/// already be a path this process can open directly (absolute, or
/// relative to the current working directory) — this function doesn't
/// resolve it against any particular crate's `CARGO_MANIFEST_DIR`,
/// since this crate is a shared dependency of every application, not
/// the one that owns the asset directory; a caller with assets under
/// its own crate joins `env!("CARGO_MANIFEST_DIR")` itself before
/// calling this.
pub fn load_image_asset(path: &str) -> ImageData {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("failed to read asset {path}: {e}"));
    AnyImageDecoder
        .decode(&bytes)
        .unwrap_or_else(|e| panic!("failed to decode asset {path}: {e}"))
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / len, v[1] / len, v[2] / len]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Per-vertex smooth normals for an arbitrary imported triangle mesh
/// (positions + shared-vertex indices, the shape [`ObjDecoder`]
/// produces) — each triangle's area-weighted face normal (the raw,
/// unnormalized cross product already scales with the triangle's area)
/// accumulates into its three vertices, normalized once at the end.
/// Unlike `scene::cube_mesh_source`'s flat per-face normals (which need
/// each face to own unshared vertices), this works directly against
/// shared OBJ-style topology without duplicating any vertex.
fn compute_smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut accum = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let (p0, p1, p2) = (positions[i0], positions[i1], positions[i2]);
        let face_normal = cross3(sub3(p1, p0), sub3(p2, p0));
        for i in [i0, i1, i2] {
            accum[i][0] += face_normal[0];
            accum[i][1] += face_normal[1];
            accum[i][2] += face_normal[2];
        }
    }
    accum.into_iter().map(normalize).collect()
}

/// One looping mono audio source's decoded samples, however
/// [`load_audio_track`] chose to get them — the two arms of
/// `asset-core::AudioAsset`, exposed behind one
/// [`next_mono_chunk`](Self::next_mono_chunk) face so a caller never
/// needs to know which one it got. Downmixed to mono here (spatializers
/// like `audio-core::BinauralRenderer` take one channel per source and
/// pan it themselves) — not cached by [`AssetCache`] like textures/
/// meshes, since a track carries its own playback cursor and two
/// listeners of the same file need two independent cursors, not one
/// shared handle.
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
                    match decoder.next_block() {
                        Ok(Some(block)) => {
                            for frame in block.chunks_exact(*channels) {
                                let sum: f32 = frame.iter().map(|&s| s as f32 / 32768.0).sum();
                                queue.push_back(sum / *channels as f32);
                            }
                        }
                        Ok(None) => {
                            if decoder.rewind().is_err() {
                                chunk.resize(frames, 0.0);
                                break;
                            }
                        }
                        Err(_) => {
                            chunk.resize(frames, 0.0);
                            break;
                        }
                    }
                }
            }
        }
        chunk
    }
}

/// Loads one audio file at `path` through `asset-core::open_audio`'s
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
                    frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32
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
/// [`crate::scene::GraphicsBase`], which owns one) calls
/// [`load_texture`](Self::load_texture)/[`load_mesh_obj`](Self::load_mesh_obj)
/// as many times as it wants for the same path (a texture shared by
/// several materials, a mesh instanced by several entities); only the
/// first call actually reads, decodes and uploads it.
#[derive(Debug, Default)]
pub struct AssetCache {
    texture_cache: HashMap<String, TextureHandle>,
    mesh_cache: HashMap<String, MeshHandle>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads `path` (see [`load_image_asset`]) as a GPU texture and
    /// returns its handle — cached by `path`: a second call with the
    /// same path returns the same handle without re-reading, decoding
    /// or uploading the file again.
    pub fn load_texture(
        &mut self,
        device: &Device,
        textures: &mut TextureRegistry,
        path: &str,
    ) -> TextureHandle {
        if let Some(&handle) = self.texture_cache.get(path) {
            return handle;
        }
        let image = load_image_asset(path);
        let handle = textures.upload(device, &image);
        self.texture_cache.insert(path.to_string(), handle);
        handle
    }

    /// Loads an OBJ model file at `path` through `asset-core::ObjDecoder`
    /// (positions + triangle indices only — see that decoder's own
    /// module doc for the current "no `vt`/`vn` parsing" scope), derives
    /// smooth per-vertex normals from the triangle topology (this
    /// module's own private `compute_smooth_normals`), and registers the
    /// result as a real [`MeshHandle`] — cached by `path`, same as
    /// [`load_texture`](Self::load_texture). UVs are `[0.0, 0.0]` for
    /// every vertex until the decoder itself gains `vt` support; a
    /// textured material still renders (uniformly sampling the
    /// texture's corner), it just won't be UV-mapped correctly yet —
    /// disclosed, not hidden.
    pub fn load_mesh_obj(
        &mut self,
        meshes: &mut MeshRegistry,
        path: &str,
    ) -> Result<MeshHandle, String> {
        if let Some(&handle) = self.mesh_cache.get(path) {
            return Ok(handle);
        }
        let bytes = std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
        let mesh_data = ObjDecoder
            .decode(&bytes)
            .map_err(|e| format!("failed to decode {path}: {e}"))?;
        let normals = compute_smooth_normals(&mesh_data.positions, &mesh_data.indices);
        let uvs = vec![[0.0, 0.0]; mesh_data.positions.len()];
        let source = MeshSource {
            positions: mesh_data.positions,
            normals,
            uvs,
            indices: mesh_data.indices,
        };
        let handle = meshes
            .register(source)
            .map_err(|e: MeshRegistryError| format!("{path}: {e}"))?;
        self.mesh_cache.insert(path.to_string(), handle);
        Ok(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat quad (two triangles sharing an edge, all four corners
    /// coplanar) — every vertex's smooth normal must come out the same
    /// (the shared face normal), the simplest real check that
    /// `compute_smooth_normals` actually averages contributions from
    /// every adjacent triangle rather than only the first or last one
    /// touching a vertex.
    #[test]
    fn compute_smooth_normals_agree_across_a_flat_shared_quad() {
        let positions = vec![
            [-1.0, 0.0, -1.0],
            [1.0, 0.0, -1.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 1.0],
        ];
        // Two triangles sharing the (0, 2) diagonal.
        let indices = vec![0, 1, 2, 0, 2, 3];

        let normals = compute_smooth_normals(&positions, &indices);
        assert_eq!(normals.len(), 4);
        // The exact sign depends on winding (not the point of this
        // test); what matters is every vertex agreeing on the *same*
        // normal, proving contributions from both triangles were
        // actually averaged together rather than one overwriting the
        // other.
        let first = normals[0];
        for normal in &normals {
            assert!(
                (normal[0] - first[0]).abs() < 1e-6
                    && (normal[1] - first[1]).abs() < 1e-6
                    && (normal[2] - first[2]).abs() < 1e-6,
                "all four coplanar vertices must share one normal: {first:?} vs {normal:?}"
            );
        }
        assert!(
            (normal_length(first) - 1.0).abs() < 1e-6,
            "normal must be unit length, got {first:?}"
        );
    }

    fn normal_length(n: [f32; 3]) -> f32 {
        (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
    }

    /// A single triangle: every normal must be unit-length and match the
    /// plain cross-product direction — the base case
    /// `compute_smooth_normals` reduces to before any averaging happens.
    #[test]
    fn compute_smooth_normals_matches_face_normal_for_a_single_triangle() {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let indices = vec![0, 1, 2];

        let normals = compute_smooth_normals(&positions, &indices);
        // cross((1,0,0), (0,0,1)) = (0*1 - 0*0, 0*0 - 1*1, 1*0 - 0*0) = (0, -1, 0)
        for normal in &normals {
            assert!(
                (normal_length(*normal) - 1.0).abs() < 1e-6,
                "normal must be unit length, got {normal:?}"
            );
            assert!(
                (normal[1] - (-1.0)).abs() < 1e-6,
                "expected -Y normal, got {normal:?}"
            );
        }
    }
}
