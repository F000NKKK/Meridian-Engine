//! GPU soft-body integration, `Fixed` flavor — the deterministic
//! counterpart to [`crate::float`]: bit-exact against
//! `physics-core::soft_body::fixed_softbody::FixedSoftBodyIntegrator::step`,
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
//! See [`crate::float`]'s module doc for the CPU-force-scatter to
//! GPU-force-gather reformulation (identical here, just over `FVec3`
//! instead of `vec3<f32>`) and [`crate`]'s module doc for why this is a
//! second, structurally distinct shader program rather than a thin alias
//! over `float`'s.

use meridian_compute_runtime::ComputeContext;
use meridian_gac_compute::fixed_wgsl::FIXED_ARITHMETIC_LIB_WGSL;
use meridian_gac_core::fixed_ga::{FixedFlavor, FixedVec3};
use meridian_gac_core::generic::Plane;
use meridian_gpu_driver::{BufferUsage, ComputePipeline, Shader};
use meridian_numeric_core::Fixed;
use meridian_physics_core::soft_body::fixed_softbody::FixedSoftBody;

use crate::generic::build_adjacency;

/// `Fixed::ONE.to_bits()` — used to build a unit vector along X (the same
/// degenerate-direction fallback `SoftBodyIntegrator::step` uses via
/// `axis_x::<F>()`) and to compute `1 / dist` via `fixed_div(ONE_BITS,
/// dist)`.
const ONE_BITS: i32 = 65536;

/// `meridian_gac_core::fixed_ga`'s own `FIXED_EPSILON` raw bits
/// (`Fixed::from_bits(4)`) — the same degenerate-length guard
/// `SoftBodyIntegrator::step` uses via `F::Scalar::EPSILON`.
const DIRECTION_EPSILON_BITS: i32 = 4;

const SOFT_BODY_STEP_WGSL_SUFFIX: &str = r#"
struct FVec3 {
    x: i32,
    y: i32,
    z: i32,
};

fn fvec3_add(a: FVec3, b: FVec3) -> FVec3 {
    return FVec3(fixed_add(a.x, b.x), fixed_add(a.y, b.y), fixed_add(a.z, b.z));
}

fn fvec3_sub(a: FVec3, b: FVec3) -> FVec3 {
    return FVec3(fixed_sub(a.x, b.x), fixed_sub(a.y, b.y), fixed_sub(a.z, b.z));
}

fn fvec3_scale(v: FVec3, s: i32) -> FVec3 {
    return FVec3(fixed_mul(v.x, s), fixed_mul(v.y, s), fixed_mul(v.z, s));
}

fn fvec3_dot(a: FVec3, b: FVec3) -> i32 {
    return fixed_add(fixed_add(fixed_mul(a.x, b.x), fixed_mul(a.y, b.y)), fixed_mul(a.z, b.z));
}

fn fvec3_length(v: FVec3) -> i32 {
    return fixed_sqrt(fvec3_dot(v, v));
}

// Flat scalar fields, not nested `FVec3` members: WGSL's uniform
// address-space layout rules pad a struct-typed member up to a 16-byte
// boundary (the same "struct boundaries align to 16" rule that gives
// `vec3<f32>` its 16-byte alignment), which `gravity`/`ground_normal`
// being a custom 3xi32 struct tripped on real hardware. Flat `i32`
// fields have ordinary 4-byte alignment, so this packs tightly with no
// surprise padding; `soft_body_step` below reassembles `FVec3`s locally.
struct Params {
    gravity_x: i32,
    gravity_y: i32,
    gravity_z: i32,
    dt: i32,
    ground_normal_x: i32,
    ground_normal_y: i32,
    ground_normal_z: i32,
    ground_d: i32,
    restitution: i32,
    particle_count: u32,
    _pad0: u32,
    _pad1: u32,
};

// Bound at 8 storage buffers total (the spec-guaranteed minimum for
// `max_storage_buffers_per_shader_stage`) — see `crate::float`'s
// `edge_params` binding comment for why the three per-half-edge arrays
// are packed into one buffer instead of three separate bindings.
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> positions_in: array<i32>;
@group(0) @binding(2) var<storage, read> velocities_in: array<i32>;
@group(0) @binding(3) var<storage, read> inverse_masses: array<i32>;
@group(0) @binding(4) var<storage, read> edge_offsets: array<u32>;
@group(0) @binding(5) var<storage, read> edge_neighbor: array<u32>;
@group(0) @binding(6) var<storage, read> edge_params: array<i32>;
@group(0) @binding(7) var<storage, read_write> positions_out: array<i32>;
@group(0) @binding(8) var<storage, read_write> velocities_out: array<i32>;

fn read_position_in(i: u32) -> FVec3 {
    return FVec3(positions_in[3u * i], positions_in[3u * i + 1u], positions_in[3u * i + 2u]);
}

fn read_velocity_in(i: u32) -> FVec3 {
    return FVec3(velocities_in[3u * i], velocities_in[3u * i + 1u], velocities_in[3u * i + 2u]);
}

fn write_position_out(i: u32, v: FVec3) {
    positions_out[3u * i] = v.x;
    positions_out[3u * i + 1u] = v.y;
    positions_out[3u * i + 2u] = v.z;
}

fn write_velocity_out(i: u32, v: FVec3) {
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

    let gravity = FVec3(params.gravity_x, params.gravity_y, params.gravity_z);
    let ground_normal = FVec3(params.ground_normal_x, params.ground_normal_y, params.ground_normal_z);

    let pos_i = read_position_in(i);
    let vel_i = read_velocity_in(i);
    let inverse_mass = inverse_masses[i];

    if (inverse_mass <= 0) {
        // Pinned particle: SoftBodyIntegrator::step never touches it.
        write_position_out(i, pos_i);
        write_velocity_out(i, vel_i);
        return;
    }

    var force = FVec3(0, 0, 0);
    let start = edge_offsets[i];
    let end = edge_offsets[i + 1u];
    for (var e: u32 = start; e < end; e = e + 1u) {
        let j = edge_neighbor[e];
        let delta = fvec3_sub(read_position_in(j), pos_i);
        let dist = fvec3_length(delta);
        var direction = FVec3(ONE_BITS, 0, 0);
        if (dist > DIRECTION_EPSILON_BITS) {
            direction = fvec3_scale(delta, fixed_div(ONE_BITS, dist));
        }
        let stretch = fixed_sub(dist, edge_rest_length[e]);
        let spring_force = fvec3_scale(direction, fixed_mul(edge_stiffness[e], stretch));

        let relative_velocity = fvec3_sub(read_velocity_in(j), vel_i);
        let closing_speed = fvec3_dot(relative_velocity, direction);
        let damping_force = fvec3_scale(direction, fixed_mul(edge_damping[e], closing_speed));

        force = fvec3_add(force, fvec3_add(spring_force, damping_force));
    }

    let acceleration = fvec3_add(fvec3_scale(force, inverse_mass), params.gravity);
    var velocity = fvec3_add(vel_i, fvec3_scale(acceleration, params.dt));
    var position = fvec3_add(pos_i, fvec3_scale(velocity, params.dt));

    let separation = fixed_add(fvec3_dot(params.ground_normal, position), params.ground_d);
    if (separation < 0) {
        position = fvec3_sub(position, fvec3_scale(params.ground_normal, separation));
        let normal_speed = fvec3_dot(velocity, params.ground_normal);
        if (normal_speed < 0) {
            let restitution_factor = fixed_add(ONE_BITS, params.restitution);
            velocity = fvec3_sub(
                velocity,
                fvec3_scale(params.ground_normal, fixed_mul(normal_speed, restitution_factor)),
            );
        }
    }

    write_position_out(i, position);
    write_velocity_out(i, velocity);
}
"#;

fn source() -> String {
    format!(
        "{lib}\n{suffix}",
        lib = FIXED_ARITHMETIC_LIB_WGSL,
        suffix = SOFT_BODY_STEP_WGSL_SUFFIX
            .replace("ONE_BITS", &ONE_BITS.to_string())
            .replace(
                "DIRECTION_EPSILON_BITS",
                &DIRECTION_EPSILON_BITS.to_string()
            )
    )
}

/// Compiled `soft_body_step` pipeline for the `Fixed` flavor — see
/// [`crate::float::SoftBodyGpuKernel`], the `f32` counterpart this
/// mirrors structurally.
#[derive(Debug)]
pub struct FixedSoftBodyGpuKernel {
    #[allow(dead_code)]
    shader: Shader,
    pipeline: ComputePipeline,
}

impl FixedSoftBodyGpuKernel {
    /// `context` must already have a GPU backend
    /// ([`ComputeContext::with_gpu`]) — panics otherwise.
    pub fn new(context: &ComputeContext) -> Self {
        let gpu = context.gpu().expect(
            "FixedSoftBodyGpuKernel::new requires a ComputeContext with a GPU backend (see ComputeContext::with_gpu)",
        );
        let shader = gpu.create_shader("soft_body_step_fixed", &source());
        let pipeline = gpu.create_compute_pipeline(&shader, "soft_body_step");
        Self { shader, pipeline }
    }

    /// Advances every particle of `body` by one timestep on the GPU and
    /// writes the result back into `body` — bit-exact against
    /// `FixedSoftBodyIntegrator::step` for the same inputs (see the
    /// module doc).
    pub async fn step(
        &self,
        context: &ComputeContext,
        body: &mut FixedSoftBody,
        gravity: FixedVec3,
        ground: Plane<FixedFlavor>,
        restitution: Fixed,
        dt: Fixed,
    ) {
        let gpu = context
            .gpu()
            .expect("FixedSoftBodyGpuKernel::step requires a ComputeContext with a GPU backend");
        let particle_count = body.particle_count();
        let adjacency = build_adjacency(body);

        let mut positions_bytes = Vec::with_capacity(particle_count * 12);
        let mut velocities_bytes = Vec::with_capacity(particle_count * 12);
        for (p, v) in body.positions.iter().zip(body.velocities.iter()) {
            for component in [p.x, p.y, p.z] {
                positions_bytes.extend_from_slice(&component.to_bits().to_le_bytes());
            }
            for component in [v.x, v.y, v.z] {
                velocities_bytes.extend_from_slice(&component.to_bits().to_le_bytes());
            }
        }
        let inverse_mass_bytes: Vec<u8> = body
            .inverse_masses
            .iter()
            .flat_map(|m| m.to_bits().to_le_bytes())
            .collect();

        let mut params_bytes = Vec::with_capacity(48);
        for component in [gravity.x, gravity.y, gravity.z, dt] {
            params_bytes.extend_from_slice(&component.to_bits().to_le_bytes());
        }
        for component in [ground.normal.x, ground.normal.y, ground.normal.z, ground.d] {
            params_bytes.extend_from_slice(&component.to_bits().to_le_bytes());
        }
        params_bytes.extend_from_slice(&restitution.to_bits().to_le_bytes());
        params_bytes.extend_from_slice(&(particle_count as u32).to_le_bytes());
        params_bytes.extend_from_slice(&0u32.to_le_bytes());
        params_bytes.extend_from_slice(&0u32.to_le_bytes());

        let edge_offsets_bytes: Vec<u8> = adjacency
            .offsets
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let edge_neighbor_bytes = pad_or(
            adjacency
                .neighbor
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect(),
            4,
        );
        let edge_rest_length_bytes = pad_or(
            adjacency
                .rest_length
                .iter()
                .flat_map(|v| v.to_bits().to_le_bytes())
                .collect(),
            4,
        );
        let edge_stiffness_bytes = pad_or(
            adjacency
                .stiffness
                .iter()
                .flat_map(|v| v.to_bits().to_le_bytes())
                .collect(),
            4,
        );
        let edge_damping_bytes = pad_or(
            adjacency
                .damping
                .iter()
                .flat_map(|v| v.to_bits().to_le_bytes())
                .collect(),
            4,
        );

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
        let edge_rest_length_buf =
            gpu.allocate_buffer(edge_rest_length_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&edge_rest_length_buf, &edge_rest_length_bytes);
        let edge_stiffness_buf =
            gpu.allocate_buffer(edge_stiffness_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&edge_stiffness_buf, &edge_stiffness_bytes);
        let edge_damping_buf = gpu.allocate_buffer(edge_damping_bytes.len(), BufferUsage::Storage);
        gpu.write_buffer(&edge_damping_buf, &edge_damping_bytes);
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
                &edge_rest_length_buf,
                &edge_stiffness_buf,
                &edge_damping_buf,
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
            p.x = Fixed::from_bits(i32::from_le_bytes(
                positions_result[base..base + 4].try_into().unwrap(),
            ));
            p.y = Fixed::from_bits(i32::from_le_bytes(
                positions_result[base + 4..base + 8].try_into().unwrap(),
            ));
            p.z = Fixed::from_bits(i32::from_le_bytes(
                positions_result[base + 8..base + 12].try_into().unwrap(),
            ));
        }
        for (i, v) in body.velocities.iter_mut().enumerate() {
            let base = i * 12;
            v.x = Fixed::from_bits(i32::from_le_bytes(
                velocities_result[base..base + 4].try_into().unwrap(),
            ));
            v.y = Fixed::from_bits(i32::from_le_bytes(
                velocities_result[base + 4..base + 8].try_into().unwrap(),
            ));
            v.z = Fixed::from_bits(i32::from_le_bytes(
                velocities_result[base + 8..base + 12].try_into().unwrap(),
            ));
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
    use meridian_physics_core::soft_body::fixed_softbody::{
        FixedSoftBodyIntegrator, fixed_icosphere_soft_body,
    };

    fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
        FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
    }

    fn ground() -> Plane<FixedFlavor> {
        Plane {
            normal: fv3(0.0, 1.0, 0.0),
            d: Fixed::ZERO,
        }
        .normalize()
    }

    fn ball(center: FixedVec3, radius: f64) -> FixedSoftBody {
        fixed_icosphere_soft_body(
            center,
            Fixed::from_num(radius),
            1,
            Fixed::from_num(0.05),
            Fixed::from_num(400.0),
            Fixed::from_num(2.0),
            Fixed::from_num(150.0),
            Fixed::from_num(1.0),
        )
    }

    /// Needs a real adapter; skip rather than fail where none is
    /// available, matching every other GPU-touching test in this
    /// workspace.
    async fn kernel_or_skip() -> Option<(ComputeContext, FixedSoftBodyGpuKernel)> {
        let context = match ComputeContext::new().with_gpu().await {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                return None;
            }
        };
        let kernel = FixedSoftBodyGpuKernel::new(&context);
        Some((context, kernel))
    }

    /// The actual point of this module: unlike `float`'s GPU/CPU
    /// agreement (numerically close), the `Fixed` GPU path must be
    /// bit-for-bit identical to `FixedSoftBodyIntegrator::step` — every
    /// position and velocity component, every step, no drift. A gentle
    /// drop at a fine timestep, matching
    /// `fixed_softbody`'s own reproducibility test, to stay inside
    /// explicit-Euler's stable regime (see that test's own comment).
    #[tokio::test]
    async fn gpu_step_matches_cpu_step_bit_exact() {
        let Some((context, kernel)) = kernel_or_skip().await else {
            return;
        };

        let mut gpu_body = ball(fv3(0.3, 0.7, -0.2), 0.5);
        let mut cpu_body = gpu_body.clone();
        let integrator =
            FixedSoftBodyIntegrator::new(fv3(0.0, -9.81, 0.0), ground(), Fixed::from_num(0.3));
        let dt = Fixed::from_num(1.0 / 240.0);

        for step in 0..60 {
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

            assert_eq!(
                gpu_body.positions, cpu_body.positions,
                "step {step}: gpu/cpu positions diverged"
            );
            assert_eq!(
                gpu_body.velocities, cpu_body.velocities,
                "step {step}: gpu/cpu velocities diverged"
            );
        }
    }
}
