//! GPU-dispatched broad-phase overlap test, the one kernel of this crate's
//! rigid-body pipeline with a real WGSL shader behind it.

use meridian_compute_driver::GpuComputeDevice;
use meridian_compute_runtime::ComputeContext;
use meridian_gac_core::FloatFlavor;
use meridian_gpu_driver::{Buffer, BufferUsage, ComputePipeline};
use meridian_physics_core::dynamics::RigidBody;

use crate::readback::le_word;

const GPU_MISSING_NEW: &str = "BroadPhaseGpuKernel::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)";
const GPU_MISSING_PAIRS: &str =
    "BroadPhaseGpuKernel::pairs requires a ComputeContext with a GPU backend";

const BROAD_PHASE_WGSL: &str = r#"
struct Params {
    pair_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> aabb_min: array<f32>;
@group(0) @binding(2) var<storage, read> aabb_max: array<f32>;
@group(0) @binding(3) var<storage, read> pair_indices: array<u32>;
@group(0) @binding(4) var<storage, read_write> overlaps_out: array<u32>;

@compute @workgroup_size(64)
fn broad_phase_overlap(@builtin(global_invocation_id) id: vec3<u32>) {
    let k = id.x;
    if (k >= params.pair_count) {
        return;
    }

    let i = pair_indices[2u * k];
    let j = pair_indices[2u * k + 1u];

    let a_min = vec3<f32>(aabb_min[3u * i], aabb_min[3u * i + 1u], aabb_min[3u * i + 2u]);
    let a_max = vec3<f32>(aabb_max[3u * i], aabb_max[3u * i + 1u], aabb_max[3u * i + 2u]);
    let b_min = vec3<f32>(aabb_min[3u * j], aabb_min[3u * j + 1u], aabb_min[3u * j + 2u]);
    let b_max = vec3<f32>(aabb_max[3u * j], aabb_max[3u * j + 1u], aabb_max[3u * j + 2u]);

    let overlaps = a_min.x <= b_max.x && a_max.x >= b_min.x
        && a_min.y <= b_max.y && a_max.y >= b_min.y
        && a_min.z <= b_max.z && a_max.z >= b_min.z;

    overlaps_out[k] = select(0u, 1u, overlaps);
}
"#;

/// GPU-dispatched counterpart of [`crate::broad_phase::BroadPhasePairsKernel`]
/// — the first (and, as of this writing, only) kernel in this crate's
/// rigid-body pipeline with a real WGSL shader behind it, not just
/// `ComputeContext::parallel_for`'s CPU backend. Deliberately scoped to
/// just the pairwise overlap test: [`RigidBody::aabb`] (reused, never
/// reimplemented — see CLAUDE.md's "don't drag another crate's logic
/// into your own" rule) is still computed on the CPU per body before
/// upload, since deriving it for a [`meridian_physics_core::collision::ColliderShape::Cuboid`]
/// needs the body's rotor sandwich-transformed local axes — genuine GA
/// (geometric-algebra) work this crate's other three pipeline stages
/// (integrator's rotor exponential/motor composition, narrow-phase's
/// SAT/manifold clipping, the solver's Gauss-Seidel passes) also still
/// do on the CPU, and porting *that* math to WGSL faithfully is real,
/// separate follow-up work, not bundled into this kernel. Once every
/// body's AABB is six plain floats, though, the O(n²) pairwise overlap
/// test itself is pure arithmetic with no GA content at all — exactly
/// the kind of embarrassingly-parallel, easy-to-verify work a GPU
/// dispatch is safe to take on first.
#[derive(Debug)]
pub struct BroadPhaseGpuKernel {
    pipeline: ComputePipeline,
}

impl BroadPhaseGpuKernel {
    /// `context` must already have a GPU backend
    /// ([`ComputeContext::with_gpu`]) — panics otherwise, the same
    /// policy [`crate::soft_body::float::SoftBodyGpuKernel::new`] uses.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context.gpu::<GpuComputeDevice>().expect(GPU_MISSING_NEW);
        let shader = gpu.create_shader("broad_phase_overlap_f32", BROAD_PHASE_WGSL);
        let pipeline = gpu.create_compute_pipeline(&shader, "broad_phase_overlap");
        Self { pipeline }
    }

    /// Every `(i, j)`, `i < j` candidate pair among `bodies` that
    /// overlaps in world-space AABB, in the same ascending order
    /// [`super::BroadPhasePairsKernel::pairs`]/
    /// `BroadPhase::find_candidate_pairs` produce — a drop-in GPU
    /// replacement for either, not just numerically similar.
    pub async fn pairs(
        &self,
        context: &ComputeContext,
        bodies: &[RigidBody<FloatFlavor>],
    ) -> Vec<(usize, usize)> {
        let candidates = candidate_pairs(bodies.len());
        if candidates.is_empty() {
            return Vec::new();
        }
        let overlaps = self.overlap_flags(context, bodies, &candidates).await;
        candidates
            .into_iter()
            .enumerate()
            .filter(|(k, _)| u32::from_le_bytes(le_word(&overlaps, k * 4)) != 0)
            .map(|(_, (i, j))| (i as usize, j as usize))
            .collect()
    }

    /// Uploads the AABBs and pair list, dispatches one thread per pair and
    /// reads back one `u32` flag per pair.
    async fn overlap_flags(
        &self,
        context: &ComputeContext,
        bodies: &[RigidBody<FloatFlavor>],
        candidates: &[(u32, u32)],
    ) -> Vec<u8> {
        let gpu = context.gpu::<GpuComputeDevice>().expect(GPU_MISSING_PAIRS);
        let (aabb_min_bytes, aabb_max_bytes) = aabb_bytes(bodies);
        let pair_count = candidates.len() as u32;
        let pair_indices_bytes: Vec<u8> = candidates
            .iter()
            .flat_map(|&(i, j)| [i.to_le_bytes(), j.to_le_bytes()])
            .flatten()
            .collect();
        let mut params_bytes = Vec::with_capacity(16);
        for word in [pair_count, 0, 0, 0] {
            params_bytes.extend_from_slice(&word.to_le_bytes());
        }

        let upload = |bytes: &[u8], usage: BufferUsage| -> Buffer {
            let buffer = gpu.allocate_buffer(bytes.len(), usage);
            gpu.write_buffer(&buffer, bytes);
            buffer
        };
        let params_buf = upload(&params_bytes, BufferUsage::Uniform);
        let aabb_min_buf = upload(&aabb_min_bytes, BufferUsage::Storage);
        let aabb_max_buf = upload(&aabb_max_bytes, BufferUsage::Storage);
        let pair_indices_buf = upload(&pair_indices_bytes, BufferUsage::Storage);
        let overlaps_out_buf = gpu.allocate_buffer((pair_count as usize) * 4, BufferUsage::Storage);

        let device = gpu.gpu_driver_device();
        let bind_group = device.create_bind_group(
            &self.pipeline.bind_group_layout(),
            &[
                &params_buf,
                &aabb_min_buf,
                &aabb_max_buf,
                &pair_indices_buf,
                &overlaps_out_buf,
            ],
        );
        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            &self.pipeline,
            &bind_group,
            pair_count.div_ceil(64).max(1),
        );
        commands.submit();
        gpu.read_buffer(&overlaps_out_buf).await
    }
}

/// Every `(i, j)` with `i < j` among `n` bodies, in ascending order.
fn candidate_pairs(n: usize) -> Vec<(u32, u32)> {
    let mut pairs = Vec::with_capacity(n.saturating_mul(n) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            pairs.push((i as u32, j as u32));
        }
    }
    pairs
}

/// The bodies' world-space AABB minima and maxima as packed `f32` triples.
fn aabb_bytes(bodies: &[RigidBody<FloatFlavor>]) -> (Vec<u8>, Vec<u8>) {
    let mut mins = Vec::with_capacity(bodies.len() * 12);
    let mut maxs = Vec::with_capacity(bodies.len() * 12);
    for body in bodies {
        let aabb = body.aabb();
        for component in [aabb.min.x, aabb.min.y, aabb.min.z] {
            mins.extend_from_slice(&component.to_le_bytes());
        }
        for component in [aabb.max.x, aabb.max.y, aabb.max.z] {
            maxs.extend_from_slice(&component.to_le_bytes());
        }
    }
    (mins, maxs)
}
