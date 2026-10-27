//! GPU soft-body integration, `float` flavor — mirrors
//! `physics-core::soft_body::float::SoftBodyIntegrator::step`'s
//! per-particle result, run as one WGSL compute invocation per particle
//! instead of a sequential CPU loop.
//!
//! The reformulation from "iterate springs, scatter into two endpoints"
//! (the CPU version) to "iterate particles, gather from own neighbors"
//! (this kernel, via [`super::csr::build_adjacency`]) produces the
//! identical per-particle net force: for a spring between `a` and `b`
//! with direction `dir_ab` (unit vector from `a` toward `b`), the CPU
//! version applies `+dir_ab * (k * stretch)` to `a` and `-dir_ab * (k *
//! stretch)` to `b`. Read from `b`'s own adjacency entry for that same
//! spring, this kernel computes `dir_ba = -dir_ab` and applies
//! `+dir_ba * (k * stretch) = -dir_ab * (k * stretch)` — exactly `b`'s
//! share. The damping term is symmetric the same way (`closing_speed` is
//! sign-invariant under swapping both the relative-velocity and
//! direction terms together). Both endpoints read `positions_in`/
//! `velocities_in` — the state at the *start* of the step, never a value
//! another invocation already wrote this dispatch — so there's no data
//! race and no ordering dependency between particles, matching the CPU
//! version's own "compute every force from the pre-step state, then
//! integrate" structure.

use meridian_compute_driver::GpuComputeDevice;
use meridian_compute_runtime::ComputeContext;
use meridian_gac_core::{FloatFlavor, Vec3};
use meridian_gpu_driver::ComputePipeline;
use meridian_physics_core::soft_body::float::{SoftBody, SoftBodyIntegrator};

use super::csr::build_adjacency;
use super::dispatch::run;
use super::float_wgsl::source;
use super::step_inputs::{
    StepInputs, Word, edge_bytes, pack_vec3s, params_bytes, unpack_vec3s, words_to_bytes,
};

const GPU_MISSING_NEW: &str = "SoftBodyGpuKernel::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)";
const GPU_MISSING_STEP: &str =
    "SoftBodyGpuKernel::step requires a ComputeContext with a GPU backend";

fn word(scalar: f32) -> Word {
    scalar.to_le_bytes()
}

fn vec3_words(v: Vec3) -> [Word; 3] {
    [word(v.x), word(v.y), word(v.z)]
}

fn vec3_from_words(words: [Word; 3]) -> Vec3 {
    Vec3::new(
        f32::from_le_bytes(words[0]),
        f32::from_le_bytes(words[1]),
        f32::from_le_bytes(words[2]),
    )
}

/// Compiled `soft_body_step` pipeline, built once against a
/// [`ComputeContext`]'s GPU backend and reused across [`Self::step`]
/// calls (topology/state buffers are still rebuilt every call — see
/// [`crate`]'s module doc).
#[derive(Debug)]
pub struct SoftBodyGpuKernel {
    pipeline: ComputePipeline,
}

impl SoftBodyGpuKernel {
    /// `context` must already have a GPU backend
    /// ([`ComputeContext::with_gpu`]) — panics otherwise, the same
    /// "caller opted into GPU dispatch explicitly" policy
    /// `meridian_gac_compute::fixed_wgsl::FixedArithmeticKernels::new` uses.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context.gpu::<GpuComputeDevice>().expect(GPU_MISSING_NEW);
        let shader = gpu.create_shader("soft_body_step_f32", &source());
        let pipeline = gpu.create_compute_pipeline(&shader, "soft_body_step");
        Self { pipeline }
    }

    /// Advances every particle of `body` by one timestep `dt` on the GPU,
    /// using `integrator`'s gravity, ground plane and restitution, and
    /// writes the result back into `body` — the GPU-dispatched
    /// counterpart of `SoftBodyIntegrator::step`. Numerically equivalent,
    /// not bit-exact (see [`crate`]'s module doc for why only the
    /// `Fixed` flavor promises that).
    pub async fn step(
        &self,
        context: &ComputeContext,
        body: &mut SoftBody,
        integrator: &SoftBodyIntegrator,
        dt: f32,
    ) {
        let inputs = step_inputs(body, integrator, dt);
        let (new_positions, new_velocities) =
            run(&self.pipeline, context, &inputs, GPU_MISSING_STEP).await;
        let count = inputs.particle_count;
        body.positions = unpack_vec3s(&new_positions, count, vec3_from_words);
        body.velocities = unpack_vec3s(&new_velocities, count, vec3_from_words);
    }
}

fn step_inputs(body: &SoftBody, integrator: &SoftBodyIntegrator, dt: f32) -> StepInputs {
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
        edge_bytes::<FloatFlavor>(&build_adjacency(body), word);
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
