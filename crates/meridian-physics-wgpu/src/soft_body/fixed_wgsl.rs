//! The WGSL program of the `Fixed` soft-body step kernel: the shared
//! fixed-point arithmetic library plus `FVec3` helpers.

use meridian_gac_wgpu::fixed_wgsl::FIXED_ARITHMETIC_LIB_WGSL;

/// `Fixed::ONE.to_bits()` — used to build a unit vector along X (the same
/// degenerate-direction fallback `SoftBodyIntegrator::step` uses via
/// `axis_x::<F>()`) and to compute `1 / dist` via `fixed_div(ONE_BITS,
/// dist)`.
pub(super) const ONE_BITS: i32 = 65536;

/// `meridian_gac::flavor::fixed`'s own `FIXED_EPSILON` raw bits
/// (`Fixed::from_bits(4)`) — the same degenerate-length guard
/// `SoftBodyIntegrator::step` uses via `F::Scalar::EPSILON`.
pub(super) const DIRECTION_EPSILON_BITS: i32 = 4;

pub(super) const SOFT_BODY_STEP_WGSL_SUFFIX: &str = r#"
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
// `max_storage_buffers_per_shader_stage`) — see `crate::soft_body::float`'s
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
        let encoded = edge_neighbor[e];
        let j = encoded & 0x7FFFFFFFu;
        let is_a = (encoded & 0x80000000u) != 0u;

        // Canonical a-to-b direction/velocity, computed identically by
        // both endpoints (subtraction negation is exact, unlike
        // `Fixed::mul`'s truncating negation) — see this crate's
        // `soft_body::csr` module doc for why recomputing per-endpoint and
        // negating the *direction* before multiplying is not bit-exact,
        // and why negating the finished `total` instead is.
        let pos_j = read_position_in(j);
        var delta = fvec3_sub(pos_i, pos_j);
        if (is_a) {
            delta = fvec3_sub(pos_j, pos_i);
        }
        let dist = fvec3_length(delta);
        var direction = FVec3(ONE_BITS, 0, 0);
        if (dist > DIRECTION_EPSILON_BITS) {
            direction = fvec3_scale(delta, fixed_div(ONE_BITS, dist));
        }
        let rest_length = edge_params[3u * e];
        let stiffness = edge_params[3u * e + 1u];
        let damping = edge_params[3u * e + 2u];

        let stretch = fixed_sub(dist, rest_length);
        let spring_force = fvec3_scale(direction, fixed_mul(stiffness, stretch));

        let vel_j = read_velocity_in(j);
        var relative_velocity = fvec3_sub(vel_i, vel_j);
        if (is_a) {
            relative_velocity = fvec3_sub(vel_j, vel_i);
        }
        let closing_speed = fvec3_dot(relative_velocity, direction);
        let damping_force = fvec3_scale(direction, fixed_mul(damping, closing_speed));

        let total = fvec3_add(spring_force, damping_force);
        var contribution = FVec3(-total.x, -total.y, -total.z);
        if (is_a) {
            contribution = total;
        }
        force = fvec3_add(force, contribution);
    }

    let acceleration = fvec3_add(fvec3_scale(force, inverse_mass), gravity);
    var velocity = fvec3_add(vel_i, fvec3_scale(acceleration, params.dt));
    var position = fvec3_add(pos_i, fvec3_scale(velocity, params.dt));

    let separation = fixed_add(fvec3_dot(ground_normal, position), params.ground_d);
    if (separation < 0) {
        position = fvec3_sub(position, fvec3_scale(ground_normal, separation));
        let normal_speed = fvec3_dot(velocity, ground_normal);
        if (normal_speed < 0) {
            let restitution_factor = fixed_add(ONE_BITS, params.restitution);
            velocity = fvec3_sub(
                velocity,
                fvec3_scale(ground_normal, fixed_mul(normal_speed, restitution_factor)),
            );
        }
    }

    write_position_out(i, position);
    write_velocity_out(i, velocity);
}
"#;

pub(super) fn source() -> String {
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
