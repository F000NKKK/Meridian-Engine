//! The WGSL program of the `f32` soft-body step kernel.

/// Matches `meridian_numeric_core::EPSILON` — the same degenerate-length
/// guard `SoftBodyIntegrator::step` uses via `F::Scalar::EPSILON`.
pub(super) const DIRECTION_EPSILON: f32 = 1e-5;

pub(super) const SOFT_BODY_STEP_WGSL: &str = r#"
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
        // `edge_neighbor[e]`'s top bit is the `is_a` flag `crate::soft_body::csr`
        // encodes (see that module's doc) — mask it off for the real
        // index. The canonical a-to-b direction/`total`-then-negate
        // shape below isn't load-bearing for `f32` (IEEE-754 multiply is
        // exactly negation-symmetric, unlike `Fixed::mul` — see
        // `crate::soft_body::csr`'s doc for the flavor this *does* matter for),
        // kept identical to `crate::soft_body::fixed`'s kernel anyway for one
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

pub(super) fn source() -> String {
    SOFT_BODY_STEP_WGSL.replace("DIRECTION_EPSILON", &format!("{DIRECTION_EPSILON}"))
}
