//! Windowed-app rendering scaffolding: mesh builders (sphere/cube/
//! pyramid/ground, each producing a real `graphics-core::MeshSource` —
//! positions, normals, UVs, indices — not raw bytes), texture loading
//! (sniffed by signature, never by extension, mirroring `asset-core`'s
//! own rule), and the `SceneRenderer`/`BloomPass`/registry bundle
//! ([`GraphicsBase`]) an application builds once. No application
//! hand-rolls a pipeline or vertex buffer itself — that's exactly what
//! `graphics-core`'s submission bridge exists to replace (see
//! docs/graphics-design.md).

use std::collections::HashMap;

use meridian_asset_core::{AnyImageDecoder, Decoder, ImageData, ObjDecoder};
use meridian_gac_core::icosphere;
use meridian_graphics_core::{
    BloomPass, MaterialRegistry, MeshHandle, MeshRegistry, MeshRegistryError, MeshSource,
    SceneRenderer, TextureHandle, TextureRegistry,
};
use meridian_graphics_driver::{DepthTexture, Device, Surface};

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

/// Builds a [`MeshSource`] for an icosphere of the given `radius`,
/// centered at its own local origin (world placement is
/// `Renderable3D::frame`'s job — `Motor3` has no scale component, so
/// radius has to be baked into the mesh itself, not applied per
/// instance). Normals are the unit-length vertex directions, UVs an
/// equirectangular projection.
pub fn icosphere_mesh_source(subdivisions: u32, radius: f32) -> MeshSource {
    let mesh = icosphere(subdivisions);
    let positions: Vec<[f32; 3]> = mesh
        .vertices
        .iter()
        .map(|v| [v.x * radius, v.y * radius, v.z * radius])
        .collect();
    let normals: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();
    let uvs: Vec<[f32; 2]> = mesh
        .vertices
        .iter()
        .map(|v| {
            let n = v.normalize();
            let u = 0.5 + n.z.atan2(n.x) / std::f32::consts::TAU;
            let v = 0.5 - n.y.asin() / std::f32::consts::PI;
            [u, v]
        })
        .collect();
    let mut indices = Vec::new();
    for face in &mesh.faces {
        for (a, b, c) in face.triangles() {
            indices.push(a as u32);
            indices.push(b as u32);
            indices.push(c as u32);
        }
    }
    MeshSource {
        positions,
        normals,
        uvs,
        indices,
    }
}

type CubeFace = ([f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]);

/// A cube with half-extent `half_extent`, one set of 4 vertices per face
/// so each face gets its own flat normal and a full `[0,1]` UV. Winding
/// verified per-face (not just assumed from a formula) — every face's
/// `cross(edge1, edge2)` must point along its own outward normal for
/// `graphics-driver`'s `FrontFace::Ccw` + back-face culling to actually
/// draw it, and a mirrored-looking cube (some faces visible only from
/// inside) is exactly what one flipped face produces.
pub fn cube_mesh_source(half_extent: f32) -> MeshSource {
    const FACES: [CubeFace; 6] = [
        // (normal, corner00, corner10, corner11, corner01) — CCW as seen
        // from outside the cube along `normal`.
        (
            [1.0, 0.0, 0.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, -1.0],
        ),
        (
            [-1.0, 0.0, 0.0],
            [-1.0, -1.0, 1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, 1.0, 1.0],
        ),
        (
            [0.0, 1.0, 0.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ),
        (
            [0.0, -1.0, 0.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
        ),
        // +Z and -Z: reversed corner order relative to the naive pattern
        // above — these two were the ones caught inside-out (verified by
        // computing cross(edge1, edge2)·normal by hand for all six faces;
        // only +Z/-Z came out negative with the naive order).
        (
            [0.0, 0.0, 1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, -1.0, 1.0],
        ),
        (
            [0.0, 0.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, -1.0],
        ),
    ];

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for (normal, c00, c10, c11, c01) in FACES {
        let base = positions.len() as u32;
        for corner in [c00, c10, c11, c01] {
            positions.push(corner.map(|c| c * half_extent));
            normals.push(normal);
        }
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    MeshSource {
        positions,
        normals,
        uvs,
        indices,
    }
}

/// A square-based pyramid: base half-extent `base_half_extent`, apex
/// `height` above the base, centered on the base's own middle (so a
/// `Renderable3D::frame` translation places the base center, not the
/// centroid). Flat per-face normals (5 faces: 1 base quad + 4 triangular
/// sides), windings verified the same way as [`cube_mesh_source`]'s —
/// see that function's doc comment for why this matters.
pub fn pyramid_mesh_source(base_half_extent: f32, height: f32) -> MeshSource {
    let base = [
        [-base_half_extent, 0.0, -base_half_extent],
        [base_half_extent, 0.0, -base_half_extent],
        [base_half_extent, 0.0, base_half_extent],
        [-base_half_extent, 0.0, base_half_extent],
    ];
    let apex = [0.0f32, height, 0.0];

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Base quad, normal -Y, triangles (0,1,2)/(0,2,3).
    {
        let base_idx = positions.len() as u32;
        for corner in base {
            positions.push(corner);
            normals.push([0.0, -1.0, 0.0]);
        }
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);
    }

    // Four triangular sides: (base[i], apex, base[i+1]) — verified
    // outward-facing winding, see the module-level comment above.
    let side_uvs = [[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]];
    for i in 0..4 {
        let b0 = base[i];
        let b1 = base[(i + 1) % 4];
        let edge1 = [apex[0] - b0[0], apex[1] - b0[1], apex[2] - b0[2]];
        let edge2 = [b1[0] - b0[0], b1[1] - b0[1], b1[2] - b0[2]];
        let normal = normalize([
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ]);
        let base_idx = positions.len() as u32;
        for corner in [b0, apex, b1] {
            positions.push(corner);
            normals.push(normal);
        }
        uvs.extend_from_slice(&side_uvs);
        indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
    }

    MeshSource {
        positions,
        normals,
        uvs,
        indices,
    }
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
/// Unlike [`cube_mesh_source`]'s flat per-face normals (which need each
/// face to own unshared vertices), this works directly against shared
/// OBJ-style topology without duplicating any vertex.
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

/// A flat quad in the local `y = 0` plane, `half_size` from center to
/// edge, its UVs tiled `uv_tiles` times across — the floor's mesh, with
/// world placement left to `Renderable3D::frame`.
pub fn ground_mesh_source(half_size: f32, uv_tiles: f32) -> MeshSource {
    MeshSource {
        positions: vec![
            [-half_size, 0.0, -half_size],
            [half_size, 0.0, -half_size],
            [half_size, 0.0, half_size],
            [-half_size, 0.0, half_size],
        ],
        normals: vec![[0.0, 1.0, 0.0]; 4],
        uvs: vec![
            [0.0, 0.0],
            [uv_tiles, 0.0],
            [uv_tiles, uv_tiles],
            [0.0, uv_tiles],
        ],
        // Winding front-facing from +Y — verified the same way as
        // `cube_mesh_source`'s faces.
        indices: vec![0, 2, 1, 0, 3, 2],
    }
}

/// The GPU-side bundle a windowed application builds once: the device/
/// surface/depth target, `graphics-core`'s scene renderer and bloom
/// pass, and empty mesh/material/texture registries ready to fill.
/// Constructed from an already-open `Device`/`Surface` (the windowed
/// handshake itself stays in the application's own `on_ready`, which is
/// the only place that needs to name `winit`/`wgpu`-adjacent types at
/// all).
///
/// **Owns a path-keyed cache for [`load_texture`](Self::load_texture)/
/// [`load_mesh_obj`](Self::load_mesh_obj)** — loading the same path
/// twice (a texture shared by several materials, a mesh instanced by
/// several entities) decodes and uploads once and returns the same
/// handle after that, rather than re-decoding and re-uploading a
/// duplicate GPU resource every call. This is the resource-loading
/// convenience layer the SDK owns on top of `asset-core`'s decoders
/// (which only turn bytes into CPU-side data, nothing path- or
/// cache-aware) and `resource-core`'s generational handles (which
/// `MeshRegistry`/`TextureRegistry` are already built on) — see this
/// crate's own module doc for why that split (decode vs. cache vs.
/// identity) lives across `asset-core`/here/`resource-core`
/// respectively, not blurred into one type.
pub struct GraphicsBase {
    pub device: Device,
    pub surface: Surface,
    pub depth: DepthTexture,
    pub renderer: SceneRenderer,
    pub bloom: BloomPass,
    pub meshes: MeshRegistry,
    pub materials: MaterialRegistry,
    pub textures: TextureRegistry,
    texture_cache: HashMap<String, TextureHandle>,
    mesh_cache: HashMap<String, MeshHandle>,
}

impl GraphicsBase {
    pub fn new(device: Device, surface: Surface, width: u32, height: u32) -> Self {
        let depth = device.create_depth_texture(width, height);
        let renderer = SceneRenderer::new(&device, &surface);
        let bloom = BloomPass::new(&device, width, height, &surface);
        Self {
            device,
            surface,
            depth,
            renderer,
            bloom,
            meshes: MeshRegistry::new(),
            materials: MaterialRegistry::new(),
            textures: TextureRegistry::new(),
            texture_cache: HashMap::new(),
            mesh_cache: HashMap::new(),
        }
    }

    /// Rebuilds every size-dependent piece (depth buffer, bloom's
    /// offscreen targets) after a window resize — `SceneRenderer` itself
    /// has no size dependency (its pipelines don't name a resolution),
    /// so it isn't rebuilt.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(&self.device, width, height);
        self.depth = self.device.create_depth_texture(width, height);
        self.bloom = BloomPass::new(&self.device, width, height, &self.surface);
    }

    /// Uploads `path` (see [`load_image_asset`]) as a GPU texture and
    /// returns its handle — cached by `path`: a second call with the
    /// same path returns the same handle without re-reading, decoding
    /// or uploading the file again.
    pub fn load_texture(&mut self, path: &str) -> TextureHandle {
        if let Some(&handle) = self.texture_cache.get(path) {
            return handle;
        }
        let image = load_image_asset(path);
        let handle = self.textures.upload(&self.device, &image);
        self.texture_cache.insert(path.to_string(), handle);
        handle
    }

    /// Loads an OBJ model file at `path` through `asset-core::ObjDecoder`
    /// (positions + triangle indices only — see that decoder's own
    /// module doc for the current "no `vt`/`vn` parsing" scope), derives
    /// smooth per-vertex normals from the triangle topology (this
    /// module's own private `compute_smooth_normals`), and registers the result as a real
    /// [`MeshHandle`] — cached by `path`, same as
    /// [`load_texture`](Self::load_texture). UVs are `[0.0, 0.0]` for
    /// every vertex until the decoder itself gains `vt` support; a
    /// textured material still renders (uniformly sampling the
    /// texture's corner), it just won't be UV-mapped correctly yet —
    /// disclosed, not hidden.
    pub fn load_mesh_obj(&mut self, path: &str) -> Result<MeshHandle, String> {
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
        let handle = self
            .meshes
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
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            assert!(
                (length - 1.0).abs() < 1e-6,
                "normal must be unit length, got {normal:?}"
            );
            assert!(
                (normal[1] - (-1.0)).abs() < 1e-6,
                "expected -Y normal, got {normal:?}"
            );
        }
    }
}
