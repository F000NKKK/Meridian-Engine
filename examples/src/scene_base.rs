//! Shared base for `magic_figures` and `physic_figures`: mesh builders
//! (sphere/cube/pyramid/ground, each producing a real
//! `graphics-core::MeshSource` — positions, normals, UVs, indices — not
//! raw bytes), texture loading from real asset files under
//! `examples/assets/textures/` (sniffed by signature, never by
//! extension, mirroring `asset-core`'s own rule), and the
//! `SceneRenderer`/`BloomPass`/registry bundle both examples build
//! identically. Neither example hand-rolls a pipeline or vertex buffer
//! itself — that's exactly what `graphics-core`'s submission bridge
//! exists to replace (see docs/graphics-design.md).

use meridian_asset_core::{AnyImageDecoder, Decoder, ImageData};
use meridian_gac_core::{Vec3, icosphere};
use meridian_graphics_core::{
    BloomPass, MaterialRegistry, MeshRegistry, MeshSource, SceneRenderer, TextureRegistry,
};
use meridian_graphics_driver::{DepthTexture, Device, Surface};

/// Reads and decodes a real image asset file, identified by its magic
/// bytes (never its extension — the same rule
/// `asset-core::AudioFormat::detect` follows for audio). `relative_path`
/// is relative to this crate's own directory (`examples/`), so callers
/// write e.g. `"assets/textures/floor.png"`.
pub fn load_image_asset(relative_path: &str) -> ImageData {
    let full_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative_path);
    let bytes = std::fs::read(&full_path)
        .unwrap_or_else(|e| panic!("failed to read asset {full_path}: {e}"));
    AnyImageDecoder
        .decode(&bytes)
        .unwrap_or_else(|e| panic!("failed to decode asset {full_path}: {e}"))
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

/// The GPU-side bundle both examples build identically: the device/
/// surface/depth target, `graphics-core`'s scene renderer and bloom
/// pass, and empty mesh/material/texture registries ready to fill.
/// Constructed from an already-open `Device`/`Surface` (the windowed
/// handshake itself stays in each example's `on_ready`, which is the
/// only place that needs to name `winit`/`wgpu`-adjacent types at all).
pub struct GraphicsBase {
    pub device: Device,
    pub surface: Surface,
    pub depth: DepthTexture,
    pub renderer: SceneRenderer,
    pub bloom: BloomPass,
    pub meshes: MeshRegistry,
    pub materials: MaterialRegistry,
    pub textures: TextureRegistry,
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

    /// Uploads `relative_path` (see [`load_image_asset`]) as a GPU
    /// texture and returns its handle.
    pub fn load_texture(&mut self, relative_path: &str) -> meridian_graphics_core::TextureHandle {
        let image = load_image_asset(relative_path);
        self.textures.upload(&self.device, &image)
    }
}
