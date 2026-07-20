//! GPU soft-body integration, `float` flavor — mirrors
//! `physics-core::soft_body::float_softbody::SoftBodyIntegrator::step`'s
//! per-particle result, run as one WGSL compute invocation per particle
//! instead of a sequential CPU loop.
//!
//! The reformulation from "iterate springs, scatter into two endpoints"
//! (the CPU version) to "iterate particles, gather from own neighbors"
//! (this kernel, via [`crate::generic::build_adjacency`]) produces the
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

use meridian_compute_runtime::ComputeContext;
use meridian_gac_core::Vec3;
use meridian_gac_core::float_ga::FloatFlavor;
use meridian_gac_core::generic::Plane;
use meridian_gpu_driver::{BufferUsage, ComputePipeline, Shader};
use meridian_physics_core::soft_body::float_softbody::SoftBody;

use crate::generic::build_adjacency;

/// Matches `meridian_numeric_core::EPSILON` — the same degenerate-length
/// guard `SoftBodyIntegrator::step` uses via `F::Scalar::EPSILON`.
const DIRECTION_EPSILON: f32 = 1e-5;

const SOFT_BODY_STEP_WGSL: &str = r#"
struct Params {
    gravity: vec3<f32>,
    dt: f32,
    ground_normal: vec3<f32>,
    ground_d: f32,
    restitution: f32,
    particle_count: u32,
    _pad0: f32,
    _pad1: f32,
};

// Bound at 8 storage buffers total (the spec-guaranteed minimum for
// `max_storage_buffers_per_shader_stage`) — `edge_rest_length`/
// `edge_stiffness`/`edge_damping` are packed into one `edge_params`
// buffer (`[rest_length, stiffness, damping]` per half-edge) rather than
// three separate bindings, since 10 separate storage buffers exceeded
// that limit on real hardware.
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> positions_in: array<f32>;
@group(0) @binding(2) var<storage, read> velocities_in: array<f32>;
@group(0) @binding(3) var<storage, read> inverse_masses: array<f32>;
@group(0) @binding(4) var<storage, read> edge_offsets: array<u32>;
@group(0) @binding(5) var<storage, read> edge_neighbor: array<u32>;
@group(0) @binding(6) var<storage, read> edge_params: array<f32>;
@group(0) @binding(7) var<storage, read_write> positions_out: array<f32>;
@group(0) @binding(8) var<storage, read_write> velocities_out: array<f32>;

fn read_position_in(i: u32) -> vec3<f32> {
    return vec3<f32>(positions_in[3u * i], positions_in[3u * i + 1u], positions_in[3u * i + 2u]);
}

fn read_velocity_in(i: u32) -> vec3<f32> {
    return vec3<f32>(velocities_in[3u * i], velocities_in[3u * i + 1u], velocities_in[3u * i + 2u]);
}

fn write_position_out(i: u32, v: vec3<f32>) {
    positions_out[3u * i] = v.x;
    positions_out[3u * i + 1u] = v.y;
    positions_out[3u * i + 2u] = v.z;
}

fn write_velocity_out(i: u32, v: vec3<f32>) {
    velocities_out[3u * i] = v.x;
    velocities_out[3u * i + 1u] = v.y;
    velocities_out[3u * i + 2u] = v.z;
}

@compute @workgroup_size(64)
fn soft_body_step(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.particle_count) {
        return;
    }

    let pos_i = read_position_in(i);
    let vel_i = read_velocity_in(i);
    let inverse_mass = inverse_masses[i];

    if (inverse_mass <= 0.0) {
        // Pinned particle: SoftBodyIntegrator::step never touches it.
        write_position_out(i, pos_i);
        write_velocity_out(i, vel_i);
        return;
    }

    var force = vec3<f32>(0.0, 0.0, 0.0);
    let start = edge_offsets[i];
    let end = edge_offsets[i + 1u];
    for (var e: u32 = start; e < end; e = e + 1u) {
        // `edge_neighbor[e]`'s top bit is the `is_a` flag `crate::generic`
        // encodes (see that module's doc) — mask it off for the real
        // index. The canonical a-to-b direction/`total`-then-negate
        // shape below isn't load-bearing for `f32` (IEEE-754 multiply is
        // exactly negation-symmetric, unlike `Fixed::mul` — see
        // `crate::generic`'s doc for the flavor this *does* matter for),
        // kept identical to `crate::fixed`'s kernel anyway for one
        // less structural difference between the two.
        let encoded = edge_neighbor[e];
        let j = encoded & 0x7FFFFFFFu;
        let is_a = (encoded & 0x80000000u) != 0u;

        let pos_j = read_position_in(j);
        let delta = select(pos_i - pos_j, pos_j - pos_i, is_a);
        let dist = length(delta);
        var direction = vec3<f32>(1.0, 0.0, 0.0);
        if (dist > DIRECTION_EPSILON) {
            direction = delta / dist;
        }
        let rest_length = edge_params[3u * e];
        let stiffness = edge_params[3u * e + 1u];
        let damping = edge_params[3u * e + 2u];

        let stretch = dist - rest_length;
        let spring_force = direction * (stiffness * stretch);

        let vel_j = read_velocity_in(j);
        let relative_velocity = select(vel_i - vel_j, vel_j - vel_i, is_a);
        let closing_speed = dot(relative_velocity, direction);
        let damping_force = direction * (damping * closing_speed);

        let total = spring_force + damping_force;
        force = force + select(-total, total, is_a);
    }

    let acceleration = force * inverse_mass + params.gravity;
    var velocity = vel_i + acceleration * params.dt;
    var position = pos_i + velocity * params.dt;

    let separation = dot(params.ground_normal, position) + params.ground_d;
    if (separation < 0.0) {
        position = position - params.ground_normal * separation;
        let normal_speed = dot(velocity, params.ground_normal);
        if (normal_speed < 0.0) {
            velocity = velocity - params.ground_normal * (normal_speed * (1.0 + params.restitution));
        }
    }

    write_position_out(i, position);
    write_velocity_out(i, velocity);
}
"#;

fn source() -> String {
    SOFT_BODY_STEP_WGSL.replace("DIRECTION_EPSILON", &format!("{DIRECTION_EPSILON}"))
}

/// Compiled `soft_body_step` pipeline, built once against a
/// [`ComputeContext`]'s GPU backend and reused across [`Self::step`]
/// calls (topology/state buffers are still rebuilt every call — see
/// [`crate`]'s module doc).
#[derive(Debug)]
pub struct SoftBodyGpuKernel {
    #[allow(dead_code)]
    shader: Shader,
    pipeline: ComputePipeline,
}

impl SoftBodyGpuKernel {
    /// `context` must already have a GPU backend
    /// ([`ComputeContext::with_gpu`]) — panics otherwise, the same
    /// "caller opted into GPU dispatch explicitly" policy
    /// `meridian_gac_compute::fixed_wgsl::FixedArithmeticKernels::new` uses.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context.gpu().expect(
            "SoftBodyGpuKernel::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)",
        );
        let shader = gpu.create_shader("soft_body_step_f32", &source());
        let pipeline = gpu.create_compute_pipeline(&shader, "soft_body_step");
        Self { shader, pipeline }
    }

    /// Advances every particle of `body` by one timestep on the GPU and
    /// writes the result back into `body` — the GPU-dispatched
    /// counterpart of `SoftBodyIntegrator::step`. Numerically equivalent,
    /// not bit-exact (see [`crate`]'s module doc for why only the
    /// `Fixed` flavor promises that).
    pub async fn step(
        &self,
        context: &ComputeContext,
        body: &mut SoftBody,
        gravity: Vec3,
        ground: Plane<FloatFlavor>,
        restitution: f32,
        dt: f32,
    ) {
        let gpu = context
            .gpu()
            .expect("SoftBodyGpuKernel::step requires a ComputeContext with a GPU backend");
        let particle_count = body.particle_count();
        let adjacency = build_adjacency(body);

        let mut positions_bytes = Vec::with_capacity(particle_count * 12);
        let mut velocities_bytes = Vec::with_capacity(particle_count * 12);
        for (p, v) in body.positions.iter().zip(body.velocities.iter()) {
            for component in [p.x, p.y, p.z] {
                positions_bytes.extend_from_slice(&component.to_le_bytes());
            }
            for component in [v.x, v.y, v.z] {
                velocities_bytes.extend_from_slice(&component.to_le_bytes());
            }
        }
        let inverse_mass_bytes: Vec<u8> = body
            .inverse_masses
            .iter()
            .flat_map(|m| m.to_le_bytes())
            .collect();

        let mut params_bytes = Vec::with_capacity(48);
        for component in [gravity.x, gravity.y, gravity.z, dt] {
            params_bytes.extend_from_slice(&component.to_le_bytes());
        }
        for component in [ground.normal.x, ground.normal.y, ground.normal.z, ground.d] {
            params_bytes.extend_from_slice(&component.to_le_bytes());
        }
        params_bytes.extend_from_slice(&restitution.to_le_bytes());
        params_bytes.extend_from_slice(&(particle_count as u32).to_le_bytes());
        params_bytes.extend_from_slice(&0f32.to_le_bytes());
        params_bytes.extend_from_slice(&0f32.to_le_bytes());

        let edge_offsets_bytes: Vec<u8> = adjacency
            .offsets
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        // A body with no springs at all (single free particle) would
        // otherwise allocate zero-length buffers, which some backends
        // reject — pad to one element; the shader never reads past
        // `edge_offsets[particle_count]`, so the padding is inert.
        let edge_neighbor_bytes: Vec<u8> = pad_or(
            adjacency
                .neighbor
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect(),
            4,
        );
        // Interleaved [rest_length, stiffness, damping] per half-edge —
        // see the WGSL `edge_params` binding's own comment for why this
        // is one buffer instead of three.
        let mut edge_params_bytes = Vec::with_capacity(adjacency.rest_length.len() * 12);
        for i in 0..adjacency.rest_length.len() {
            edge_params_bytes.extend_from_slice(&adjacency.rest_length[i].to_le_bytes());
            edge_params_bytes.extend_from_slice(&adjacency.stiffness[i].to_le_bytes());
            edge_params_bytes.extend_from_slice(&adjacency.damping[i].to_le_bytes());
        }
        let edge_params_bytes = pad_or(edge_params_bytes, 4);

        let params_buf = gpu.allocate_buffer(params_bytes.len(), BufferUsage::Uniform);
        gpu.write_buffer(&params_buf, &params_bytes);
        let positions_in = gpu.allocate_buffer(positions_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&positions_in, &positions_bytes);
        let velocities_in = gpu.allocate_buffer(velocities_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&velocities_in, &velocities_bytes);
        let inverse_masses_buf =
            gpu.allocate_buffer(inverse_mass_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&inverse_masses_buf, &inverse_mass_bytes);
        let edge_offsets_buf = gpu.allocate_buffer(edge_offsets_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&edge_offsets_buf, &edge_offsets_bytes);
        let edge_neighbor_buf =
            gpu.allocate_buffer(edge_neighbor_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&edge_neighbor_buf, &edge_neighbor_bytes);
        let edge_params_buf = gpu.allocate_buffer(edge_params_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&edge_params_buf, &edge_params_bytes);
        let positions_out = gpu.allocate_buffer(positions_bytes.len(), BufferUsage::Storage);
        let velocities_out = gpu.allocate_buffer(velocities_bytes.len(), BufferUsage::Storage);

        let device = gpu.gpu_driver_device();
        let bind_group = device.create_bind_group(
            &self.pipeline.bind_group_layout(),
            &[
                &params_buf,
                &positions_in,
                &velocities_in,
                &inverse_masses_buf,
                &edge_offsets_buf,
                &edge_neighbor_buf,
                &edge_params_buf,
                &positions_out,
                &velocities_out,
            ],
        );

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute_with_bind_group(
            &self.pipeline,
            &bind_group,
            (particle_count as u32).div_ceil(64).max(1),
        );
        commands.submit();

        let positions_result = gpu.read_buffer(&positions_out).await;
        let velocities_result = gpu.read_buffer(&velocities_out).await;

        for (i, p) in body.positions.iter_mut().enumerate() {
            let base = i * 12;
            p.x = f32::from_le_bytes(positions_result[base..base + 4].try_into().unwrap());
            p.y = f32::from_le_bytes(positions_result[base + 4..base + 8].try_into().unwrap());
            p.z = f32::from_le_bytes(positions_result[base + 8..base + 12].try_into().unwrap());
        }
        for (i, v) in body.velocities.iter_mut().enumerate() {
            let base = i * 12;
            v.x = f32::from_le_bytes(velocities_result[base..base + 4].try_into().unwrap());
            v.y = f32::from_le_bytes(velocities_result[base + 4..base + 8].try_into().unwrap());
            v.z = f32::from_le_bytes(velocities_result[base + 8..base + 12].try_into().unwrap());
        }
    }
}

fn pad_or(bytes: Vec<u8>, min_len: usize) -> Vec<u8> {
    if bytes.is_empty() {
        vec![0u8; min_len]
    } else {
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::Vec3;
    use meridian_physics_core::soft_body::float_softbody::{
        SoftBodyIntegrator, icosphere_soft_body,
    };

    fn ground() -> Plane<FloatFlavor> {
        Plane {
            normal: Vec3::Y,
            d: 0.0,
        }
        .normalize()
    }

    fn ball(center: Vec3, radius: f32) -> SoftBody {
        icosphere_soft_body(center, radius, 1, 0.05, 400.0, 2.0, 150.0, 1.0)
    }

    /// Needs a real adapter; skip rather than fail where none is
    /// available, matching every other GPU-touching test in this
    /// workspace.
    async fn kernel_or_skip() -> Option<(ComputeContext, SoftBodyGpuKernel)> {
        let context = match ComputeContext::new().with_gpu().await {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                return None;
            }
        };
        let kernel = SoftBodyGpuKernel::new(&context);
        Some((context, kernel))
    }

    /// Stepping the same scenario on the GPU and on the CPU must agree
    /// closely (not bit-exact — see the module doc) during smooth
    /// free-fall, before any ground contact. A mass-spring system is
    /// chaotic once collision response and inter-particle feedback are
    /// both in play (a single differing float ULP at the moment of
    /// impact can select a measurably different post-bounce trajectory,
    /// the same sensitivity that makes weather forecasting hard, not a
    /// bug in either integrator) — so this only asserts tight agreement
    /// for the pre-contact steps, then checks the post-contact tail
    /// stays physically plausible (bounded, no NaN/blow-up) rather than
    /// demanding it retrace the CPU path exactly.
    #[tokio::test]
    async fn gpu_step_matches_cpu_step_closely() {
        let Some((context, kernel)) = kernel_or_skip().await else {
            return;
        };

        let mut gpu_body = ball(Vec3::new(0.1, 0.6, -0.05), 0.5);
        let mut cpu_body = gpu_body.clone();
        let integrator = SoftBodyIntegrator::new(Vec3::new(0.0, -9.81 * 4.0, 0.0), ground(), 0.2);
        let dt = 1.0 / 240.0;
        let pre_contact_steps = 20;

        for step in 0..120 {
            kernel
                .step(
                    &context,
                    &mut gpu_body,
                    integrator.gravity,
                    integrator.ground,
                    integrator.restitution,
                    dt,
                )
                .await;
            integrator.step(&mut cpu_body, dt);

            for (i, (gpu_p, cpu_p)) in gpu_body
                .positions
                .iter()
                .zip(cpu_body.positions.iter())
                .enumerate()
            {
                let diff = (*gpu_p - *cpu_p).length();
                if step < pre_contact_steps {
                    assert!(
                        diff < 1e-4,
                        "step {step} particle {i} diverged during free-fall: gpu={gpu_p:?} cpu={cpu_p:?} diff={diff}"
                    );
                } else {
                    assert!(
                        gpu_p.x.is_finite()
                            && gpu_p.y.is_finite()
                            && gpu_p.z.is_finite()
                            && gpu_p.length() < 10.0,
                        "step {step} particle {i}: gpu position blew up: {gpu_p:?}"
                    );
                }
            }
        }
    }
}
