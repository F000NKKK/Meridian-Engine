//! GPU soft-body integration, `Fixed` flavor — the deterministic
//! counterpart to [`super::float`]: bit-exact against
//! `physics-core::soft_body::fixed::FixedSoftBodyIntegrator::step`,
//! not merely numerically close, the same guarantee
//! `meridian_gac_compute::fixed_wgsl::FixedArithmeticKernels` proves for
//! plain scalar arithmetic. This module reuses that same WGSL arithmetic
//! library (`fixed_add`/`fixed_sub`/`fixed_mul`/`fixed_div`/`fixed_sqrt`,
//! each a bit-exact emulation of `Fixed`'s own `i32` operations) and
//! builds a small `FVec3` (three `i32` Q16.16 raw-bit fields) WGSL struct
//! plus vector helpers (`fvec3_add`/`fvec3_dot`/`fvec3_length`/...) on top
//! — the fixed-point equivalent of WGSL's native `vec3<f32>` this module
//! doesn't get to use.
//!
//! See [`super::float`]'s module doc for the CPU-force-scatter to
//! GPU-force-gather reformulation (identical here, just over `FVec3`
//! instead of `vec3<f32>`) and [`crate`]'s module doc for why this is a
//! second, structurally distinct shader program rather than a thin alias
//! over `float`'s.

use meridian_compute_driver::GpuComputeDevice;
use meridian_compute_runtime::ComputeContext;
use meridian_gac_core::{FixedFlavor, FixedVec3};
use meridian_gpu_driver::ComputePipeline;
use meridian_numeric_core::Fixed;
use meridian_physics_core::soft_body::fixed::{FixedSoftBody, FixedSoftBodyIntegrator};

use super::csr::build_adjacency;
use super::dispatch::run;
use super::fixed_wgsl::source;
use super::step_inputs::{
    StepInputs, Word, edge_bytes, pack_vec3s, params_bytes, unpack_vec3s, words_to_bytes,
};

const GPU_MISSING_NEW: &str = "FixedSoftBodyGpuKernel::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)";
const GPU_MISSING_STEP: &str =
    "FixedSoftBodyGpuKernel::step requires a ComputeContext with a GPU backend";

fn word(scalar: Fixed) -> Word {
    scalar.to_bits().to_le_bytes()
}

fn vec3_words(v: FixedVec3) -> [Word; 3] {
    [word(v.x), word(v.y), word(v.z)]
}

fn vec3_from_words(words: [Word; 3]) -> FixedVec3 {
    let fixed = |w: Word| Fixed::from_bits(i32::from_le_bytes(w));
    FixedVec3::new(fixed(words[0]), fixed(words[1]), fixed(words[2]))
}

/// Compiled `soft_body_step` pipeline for the `Fixed` flavor — see
/// [`super::float::SoftBodyGpuKernel`], the `f32` counterpart this
/// mirrors structurally.
#[derive(Debug)]
pub struct FixedSoftBodyGpuKernel {
    pipeline: ComputePipeline,
}

impl FixedSoftBodyGpuKernel {
    /// `context` must already have a GPU backend
    /// ([`ComputeContext::with_gpu`]) — panics otherwise.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context.gpu::<GpuComputeDevice>().expect(GPU_MISSING_NEW);
        let shader = gpu.create_shader("soft_body_step_fixed", &source());
        let pipeline = gpu.create_compute_pipeline(&shader, "soft_body_step");
        Self { pipeline }
    }

    /// Advances every particle of `body` by one timestep `dt` on the GPU,
    /// using `integrator`'s gravity, ground plane and restitution, and
    /// writes the result back into `body` — bit-exact against
    /// `FixedSoftBodyIntegrator::step` for the same inputs (see the
    /// module doc).
    pub async fn step(
        &self,
        context: &ComputeContext,
        body: &mut FixedSoftBody,
        integrator: &FixedSoftBodyIntegrator,
        dt: Fixed,
    ) {
        let inputs = step_inputs(body, integrator, dt);
        let (new_positions, new_velocities) =
            run(&self.pipeline, context, &inputs, GPU_MISSING_STEP).await;
        let count = inputs.particle_count;
        body.positions = unpack_vec3s(&new_positions, count, vec3_from_words);
        body.velocities = unpack_vec3s(&new_velocities, count, vec3_from_words);
    }
}

fn step_inputs(
    body: &FixedSoftBody,
    integrator: &FixedSoftBodyIntegrator,
    dt: Fixed,
) -> StepInputs {
    let ground = integrator.ground;
    let [gx, gy, gz] = vec3_words(integrator.gravity);
    let [nx, ny, nz] = vec3_words(ground.normal);
    let scalars = [
        gx,
        gy,
        gz,
        word(dt),
        nx,
        ny,
        nz,
        word(ground.d),
        word(integrator.restitution),
    ];
    let particle_count = body.particle_count();
    let [edge_offsets, edge_neighbor, edge_params] =
        edge_bytes::<FixedFlavor>(&build_adjacency(body), word);
    StepInputs {
        params: params_bytes(scalars, particle_count),
        positions: pack_vec3s(&body.positions, vec3_words),
        velocities: pack_vec3s(&body.velocities, vec3_words),
        inverse_masses: words_to_bytes(body.inverse_masses.iter().map(|&m| word(m))),
        edge_offsets,
        edge_neighbor,
        edge_params,
        particle_count,
    }
}
