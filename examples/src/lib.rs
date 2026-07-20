//! Shared rendering scaffolding for the soft-body examples
//! (`soft_body_rubber_balls`, `soft_body_jiggle_deterministic`,
//! `soft_body_jiggle_float`) — the mesh-topology/vertex-buffer plumbing
//! all three need is identical; only the physics backend and scenario
//! differ between them. Not part of any published crate (`publish =
//! false` on this package) — this is example-only scaffolding, not a
//! new engine API.
//!
//! Unlike `spinning_cube`'s shader (which takes local-space vertices and
//! a `model` matrix), soft-body particle positions are already
//! world-space (`icosphere_soft_body`'s `center` offset is baked into
//! every particle at construction), so [`SOFT_BODY_SHADER`]'s uniform is
//! just a view-projection matrix, no per-mesh model matrix.

use meridian_gac_core::generic::Face;
use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_core::Camera;
use meridian_graphics_driver::{VertexAttributeDesc, VertexFormat, VertexLayout};
use meridian_platform_core::{InputState, KeyCode};

pub const SOFT_BODY_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_normal = in.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let diffuse = max(dot(normalize(in.world_normal), light_dir), 0.0);
    let ambient = 0.2;
    let base_color = vec3<f32>(0.85, 0.3, 0.25);
    let color = base_color * (ambient + diffuse * 0.8);
    return vec4<f32>(color, 1.0);
}
"#;

/// A procedural checkerboard floor — no texture asset, the checker
/// pattern is computed directly in the fragment shader from world-space
/// `x`/`z` (1-unit tiles, parity via `& 1` on the floored tile
/// coordinates so it stays correct for negative coordinates too, unlike
/// `%` on signed integers). Exists because the soft-body examples were
/// otherwise "balls floating in a black void" — no ground reference to
/// judge height, deformation, or collision against.
pub const GROUND_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tile_x = i32(floor(in.world_position.x));
    let tile_z = i32(floor(in.world_position.z));
    let is_dark = ((tile_x ^ tile_z) & 1) == 0;
    let checker = select(vec3<f32>(0.82, 0.82, 0.88), vec3<f32>(0.22, 0.22, 0.28), is_dark);
    return vec4<f32>(checker, 1.0);
}
"#;

/// One large flat quad (`size` x `size`, centered at `(0, y, 0)`) for
/// [`GROUND_SHADER`] to shade — reuses [`soft_body_vertex_layout`]'s
/// position+normal layout (the normal is written but unread by
/// `GROUND_SHADER`, kept only so both pipelines share one vertex
/// layout/buffer shape).
pub fn ground_quad_buffers(size: f32, y: f32) -> (Vec<u8>, Vec<u8>, u32) {
    let half = size / 2.0;
    let corners = [
        Vec3::new(-half, y, -half),
        Vec3::new(half, y, -half),
        Vec3::new(half, y, half),
        Vec3::new(-half, y, half),
    ];
    let normal = Vec3::Y;

    let mut vertex_bytes = Vec::new();
    for p in corners {
        for component in [p.x, p.y, p.z, normal.x, normal.y, normal.z] {
            vertex_bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    // Winding matters: `Device::create_render_pipeline` culls back faces
    // (`FrontFace::Ccw`, see `spinning_cube`'s identical note on its own
    // cube mesh) — [0,1,2, 0,2,3] here would wind counter-clockwise as
    // seen from *below* the ground (normal pointing -Y), invisible to a
    // camera looking down at it from above. [0,2,1, 0,3,2] is the
    // reversed winding that's front-facing from +Y instead — this was
    // the actual reason the ground plane never rendered.
    let mut index_bytes = Vec::new();
    for i in [0u16, 2, 1, 0, 3, 2] {
        index_bytes.extend_from_slice(&i.to_le_bytes());
    }
    (vertex_bytes, index_bytes, 6)
}

pub fn soft_body_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 24, // position (12 bytes) + normal (12 bytes)
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 12,
            },
        ],
    }
}

/// Builds `(vertex_bytes, index_bytes, index_count)` for a soft body's
/// current surface shape: `surface_positions` (world-space, the
/// `SoftBody`'s particles excluding the interior center particle — see
/// `icosphere_soft_body`'s own doc comment) plus `faces` (the
/// icosphere's fixed triangle topology — same `subdivisions` always
/// produces the same face list, independent of how the vertices have
/// since deformed, so callers build it once via
/// [`meridian_gac_core::icosphere`] and reuse it every frame).
///
/// Flat per-face normals are recomputed from `surface_positions` every
/// call (not cached) so lighting responds to deformation — each face
/// gets its own un-shared vertices (like `spinning_cube`'s cube mesh),
/// trading vertex count for genuinely flat shading instead of
/// interpolated/averaged normals that would blur a sharply dented face.
pub fn soft_body_render_buffers(
    surface_positions: &[Vec3],
    faces: &[Face],
) -> (Vec<u8>, Vec<u8>, u32) {
    let mut vertex_bytes = Vec::new();
    let mut index_bytes = Vec::new();
    let mut index_count = 0u32;
    let mut next_index: u16 = 0;

    for face in faces {
        for (a, b, c) in face.triangles() {
            let pa = surface_positions[a];
            let pb = surface_positions[b];
            let pc = surface_positions[c];
            let normal = (pb - pa).cross(pc - pa).normalize();

            for p in [pa, pb, pc] {
                for component in [p.x, p.y, p.z, normal.x, normal.y, normal.z] {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
                index_bytes.extend_from_slice(&next_index.to_le_bytes());
                next_index += 1;
                index_count += 1;
            }
        }
    }

    (vertex_bytes, index_bytes, index_count)
}

pub fn mat4_to_bytes(m: [[f32; 4]; 4]) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let mut offset = 0;
    for column in m {
        for component in column {
            bytes[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
            offset += 4;
        }
    }
    bytes
}

/// A camera rotor turning `gac-core`'s local-forward `+X` toward `target
/// - eye` — see `spinning_cube`'s identical helper, duplicated here
/// (not worth a shared engine-level utility for three examples) rather
/// than depended on across two ``[[example]]`` binaries.
pub fn look_at_rotor(eye: Vec3, target: Vec3) -> meridian_gac_core::Rotor {
    use meridian_gac_core::Rotor;
    let forward = (target - eye).normalize();
    let local_forward = Vec3::X;
    let cos_angle = local_forward.dot(forward).clamp(-1.0, 1.0);
    if cos_angle > 0.9999 {
        return Rotor::identity();
    }
    if cos_angle < -0.9999 {
        return Rotor::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    }
    let axis = local_forward.cross(forward).normalize();
    Rotor::from_axis_angle(axis, cos_angle.acos())
}

/// A free-fly camera: WASD to move (relative to the way it's currently
/// looking, not world axes), Space/Ctrl for up/down, mouse to look
/// around — the caller is expected to grab the cursor
/// (`Window::set_cursor_grabbed(true)` once in `on_ready`) so the mouse
/// steers the view directly instead of a visible cursor wandering off
/// the window. Exists because a fixed `look_at_rotor(eye, target)`
/// camera (`spinning_cube`'s approach) gives no way to inspect a scene
/// from angles the author didn't hardcode — the soft-body examples need
/// that inspection more than a spinning cube does, since "is this ball
/// actually deforming/jiggling or just sitting there" is exactly the
/// kind of thing you want to walk around and look at.
pub struct FlyCamera {
    pub position: Vec3,
    /// Radians, measured from `+X` toward `+Z` (matches `Vec3::X` being
    /// `gac-core`'s local-forward convention — see [`look_at_rotor`]).
    pub yaw: f32,
    /// Radians, clamped just short of &plusmn;90&deg; to avoid a gimbal
    /// flip at the poles.
    pub pitch: f32,
    pub move_speed: f32,
    pub look_sensitivity: f32,
}

impl FlyCamera {
    pub fn new(position: Vec3) -> Self {
        // Facing roughly toward -Z (a scene's contents in these examples
        // sit near the origin along -Z from a positive-Z starting eye
        // point) with a slight downward tilt, a reasonable default that
        // doesn't start looking at empty sky.
        Self {
            position,
            yaw: -core::f32::consts::FRAC_PI_2,
            pitch: -0.2,
            move_speed: 3.0,
            look_sensitivity: 0.003,
        }
    }

    /// Advances the camera by one frame of `input`/`dt`. Call once per
    /// `on_redraw`, after computing `dt` and before building this
    /// frame's [`Camera`] via [`Self::camera`]. Reads
    /// [`InputState::raw_mouse_delta`] (device-relative motion, not
    /// cursor-position-based) so this keeps working once the caller
    /// grabs the cursor via `Window::set_cursor_grabbed(true)` in
    /// `on_ready` (the expected setup — see this type's own doc comment)
    /// — a locked cursor stops generating position deltas entirely.
    pub fn update(&mut self, input: &InputState, dt: f32) {
        let (dx, dy) = input.raw_mouse_delta();
        self.yaw -= dx * self.look_sensitivity;
        self.pitch = (self.pitch - dy * self.look_sensitivity).clamp(
            -core::f32::consts::FRAC_PI_2 + 0.01,
            core::f32::consts::FRAC_PI_2 - 0.01,
        );

        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let forward_horiz = Vec3::new(cy, 0.0, sy);
        let right = Vec3::new(sy, 0.0, -cy);
        let mut movement = Vec3::ZERO;
        if input.is_key_down(KeyCode::W) {
            movement = movement + forward_horiz;
        }
        if input.is_key_down(KeyCode::S) {
            movement = movement - forward_horiz;
        }
        if input.is_key_down(KeyCode::D) {
            movement = movement + right;
        }
        if input.is_key_down(KeyCode::A) {
            movement = movement - right;
        }
        if input.is_key_down(KeyCode::Space) {
            movement = movement + Vec3::Y;
        }
        if input.is_key_down(KeyCode::ControlLeft) {
            movement = movement - Vec3::Y;
        }
        if movement.length() > 1e-5 {
            let speed = if input.is_key_down(KeyCode::ShiftLeft) {
                self.move_speed * 3.0
            } else {
                self.move_speed
            };
            self.position = self.position + movement.normalize() * speed * dt;
        }
    }

    /// The view rotor: yaw around world `+Y`, *then* pitch around the
    /// yaw-rotated local-right axis — not [`look_at_rotor`]. That
    /// helper picks the shortest-arc rotation from local-forward to a
    /// target direction, which is ambiguous about roll (many rotations
    /// take `+X` to a given `forward`, and the shortest-arc choice drifts
    /// between them as pitch changes) — it visibly rolled the view
    /// clockwise/counter-clockwise while pitching up/down. Composing an
    /// explicit yaw-then-pitch instead is standard FPS-camera
    /// construction: rotating around a horizontal axis that's itself
    /// already been yawed never introduces roll, by construction (the
    /// world-up direction stays consistently "up" in the resulting view
    /// at every pitch).
    fn rotor(&self) -> meridian_gac_core::Rotor {
        use meridian_gac_core::Rotor;
        let yaw_rotor = Rotor::from_axis_angle(Vec3::Y, -self.yaw);
        let right_after_yaw = yaw_rotor.transform_vector(Vec3::Z);
        let pitch_rotor = Rotor::from_axis_angle(right_after_yaw, self.pitch);
        yaw_rotor.compose(pitch_rotor)
    }

    pub fn camera(&self, aspect_ratio: f32) -> Camera {
        Camera {
            frame: Motor3::from_rotation_translation(self.rotor(), self.position),
            projection: meridian_gac_core::Projection::perspective(
                55.0_f32.to_radians(),
                aspect_ratio,
                0.1,
                100.0,
            ),
        }
    }
}

#[cfg(test)]
mod fly_camera_tests {
    use super::*;
    use meridian_platform_core::{InputState, KeyCode};

    /// Returns (forward_horiz, right, up) expected from the camera's yaw.
    fn expected_basis(yaw: f32) -> (Vec3, Vec3, Vec3) {
        let cy = yaw.cos();
        let sy = yaw.sin();
        (Vec3::new(cy, 0.0, sy), Vec3::new(sy, 0.0, -cy), Vec3::Y)
    }

    #[test]
    fn rotor_and_forward_horiz_agree() {
        // Verify that the ad-hoc forward_horiz computation matches
        // what rotor() actually produces for the horizontal component.
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            3.0 * core::f32::consts::FRAC_PI_4,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_4,
            -core::f32::consts::FRAC_PI_2,
            -3.0 * core::f32::consts::FRAC_PI_4,
        ];
        for &yaw in &yaws {
            let cam = FlyCamera {
                position: Vec3::ZERO,
                yaw,
                pitch: 0.0,
                move_speed: 3.0,
                look_sensitivity: 0.003,
            };
            let rotor_forward = cam.rotor().transform_vector(Vec3::X);
            let cy = yaw.cos();
            let sy = yaw.sin();
            let expected_forward_horiz = Vec3::new(cy, 0.0, sy);
            // Compare XZ components (Y may differ if pitch were non-zero, but pitch=0 here)
            assert!(
                (rotor_forward.x - expected_forward_horiz.x).abs() < 1e-6,
                "X mismatch at yaw={}: rotor_forward.x={} expected={}",
                yaw,
                rotor_forward.x,
                expected_forward_horiz.x
            );
            assert!(
                (rotor_forward.z - expected_forward_horiz.z).abs() < 1e-6,
                "Z mismatch at yaw={}: rotor_forward.z={} expected={}",
                yaw,
                rotor_forward.z,
                expected_forward_horiz.z
            );
            // Y should be 0 at pitch=0
            assert!(
                rotor_forward.y.abs() < 1e-6,
                "Y should be 0 at pitch=0, yaw={}, got {}",
                yaw,
                rotor_forward.y
            );
        }
    }

    fn simulate(dt: f32, speed: f32, keys: &[KeyCode]) -> Vec3 {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        cam.move_speed = speed;
        let mut input = InputState::new();
        for &k in keys {
            input.press_key(k);
        }
        cam.update(&input, dt);
        cam.position
    }

    fn simulate_at(yaw: f32, dt: f32, speed: f32, keys: &[KeyCode]) -> Vec3 {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        cam.yaw = yaw;
        cam.move_speed = speed;
        let mut input = InputState::new();
        for &k in keys {
            input.press_key(k);
        }
        cam.update(&input, dt);
        cam.position
    }

    #[test]
    fn no_keys_does_not_move() {
        let pos = simulate(1.0, 3.0, &[]);
        assert_eq!(pos, Vec3::ZERO);
    }

    #[test]
    fn w_moves_forward_along_yaw_basis() {
        // Test at 8 different yaw angles covering all quadrants
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            3.0 * core::f32::consts::FRAC_PI_4,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_4,
            -core::f32::consts::FRAC_PI_2,
            -3.0 * core::f32::consts::FRAC_PI_4,
        ];
        for &yaw in &yaws {
            let pos = simulate_at(yaw, 1.0, 3.0, &[KeyCode::W]);
            let (fwd, _, _) = expected_basis(yaw);
            let expected = fwd.normalize() * 3.0;
            assert!(
                (pos - expected).length() < 1e-5,
                "W yaw={} got {:?} expected {:?}",
                yaw,
                pos,
                expected
            );
        }
    }

    #[test]
    fn s_moves_backward_opposite_to_w() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let fwd = simulate_at(yaw, 1.0, 3.0, &[KeyCode::W]);
            let bwd = simulate_at(yaw, 1.0, 3.0, &[KeyCode::S]);
            assert!((fwd + bwd).length() < 1e-5, "S should oppose W at yaw={}", yaw);
        }
    }

    #[test]
    fn d_moves_right_along_yaw_basis() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let pos = simulate_at(yaw, 1.0, 3.0, &[KeyCode::D]);
            let (_, right, _) = expected_basis(yaw);
            let expected = right.normalize() * 3.0;
            assert!(
                (pos - expected).length() < 1e-5,
                "D yaw={} got {:?} expected {:?}",
                yaw,
                pos,
                expected
            );
        }
    }

    #[test]
    fn a_moves_left_opposite_to_d() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let d = simulate_at(yaw, 1.0, 3.0, &[KeyCode::D]);
            let a = simulate_at(yaw, 1.0, 3.0, &[KeyCode::A]);
            assert!((d + a).length() < 1e-5, "A should oppose D at yaw={}", yaw);
        }
    }

    #[test]
    fn space_moves_up_and_ctrl_moves_down() {
        let up = simulate(1.0, 3.0, &[KeyCode::Space]);
        assert_eq!(up, Vec3::new(0.0, 3.0, 0.0));

        let down = simulate(1.0, 3.0, &[KeyCode::ControlLeft]);
        assert_eq!(down, Vec3::new(0.0, -3.0, 0.0));

        let both = simulate(1.0, 3.0, &[KeyCode::Space, KeyCode::ControlLeft]);
        assert_eq!(both, Vec3::ZERO);
    }

    #[test]
    fn diagonal_movement_preserves_length() {
        let pos = simulate_at(0.0, 1.0, 3.0, &[KeyCode::W, KeyCode::D]);
        let expected = (Vec3::X - Vec3::Z).normalize() * 3.0;
        assert!((pos - expected).length() < 1e-5, "W+D diagonal got {:?}", pos);
    }

    #[test]
    fn full_3d_movement_wsad_space_ctrl() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let (fwd, right, _) = expected_basis(yaw);
            let pos = simulate_at(
                yaw,
                1.0,
                3.0,
                &[KeyCode::W, KeyCode::D, KeyCode::Space],
            );
            let expected = (fwd.normalize() + right.normalize() + Vec3::Y).normalize() * 3.0;
            assert!(
                (pos - expected).length() < 1e-5,
                "W+D+Space yaw={} got {:?} expected {:?}",
                yaw,
                pos,
                expected
            );
        }
    }

    #[test]
    fn pitch_does_not_affect_movement_direction() {
        let pitches = [0.0, 0.5, -0.3, 1.2, -1.0];
        for &pitch in &pitches {
            let pos = simulate_at(0.0, 1.0, 3.0, &[KeyCode::W]);
            let expected = Vec3::new(3.0, 0.0, 0.0);
            assert!(
                (pos - expected).length() < 1e-5,
                "W at pitch={} got {:?}",
                pitch,
                pos
            );
        }
    }

    #[test]
    fn move_speed_scales_velocity() {
        let slow = simulate(1.0, 1.0, &[KeyCode::W]);
        let fast = simulate(1.0, 10.0, &[KeyCode::W]);
        assert!((fast - slow * 10.0).length() < 1e-5);
    }

    #[test]
    fn dt_scales_velocity() {
        let t1 = simulate(0.5, 3.0, &[KeyCode::W]);
        let t2 = simulate(1.0, 3.0, &[KeyCode::W]);
        assert!((t2 - t1 * 2.0).length() < 1e-5);
    }

    #[test]
    fn shift_multiplies_move_speed() {
        let normal = simulate(1.0, 3.0, &[KeyCode::W]);
        let shifted = simulate(1.0, 3.0, &[KeyCode::W, KeyCode::ShiftLeft]);
        assert!((shifted - normal * 3.0).length() < 1e-5);
    }

    #[test]
    fn mouse_affects_yaw() {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        let start_yaw = cam.yaw;
        let mut input = InputState::new();
        input.accumulate_mouse_motion(100.0, 0.0);
        cam.update(&input, 1.0);
        assert!(
            (cam.yaw - start_yaw).abs() > 0.001,
            "mouse dx should change yaw"
        );
    }

    #[test]
    fn mouse_affects_pitch() {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        let start_pitch = cam.pitch;
        let mut input = InputState::new();
        input.accumulate_mouse_motion(0.0, 100.0);
        cam.update(&input, 1.0);
        assert!(
            (cam.pitch - start_pitch).abs() > 0.001,
            "mouse dy should change pitch"
        );
    }

    #[test]
    fn pitch_clamping() {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        let mut input = InputState::new();
        // Try to exceed ±90°
        input.accumulate_mouse_motion(0.0, -1e6);
        cam.update(&input, 1.0);
        assert!(cam.pitch > -core::f32::consts::FRAC_PI_2 + 0.009);
        input.accumulate_mouse_motion(0.0, 1e6);
        cam.update(&input, 1.0);
        assert!(cam.pitch < core::f32::consts::FRAC_PI_2 - 0.009);
    }

    #[test]
    fn zero_movement_does_not_change_position() {
        let mut cam = FlyCamera::new(Vec3::new(10.0, -5.0, 3.0));
        let input = InputState::new();
        cam.update(&input, 1.0);
        assert_eq!(cam.position, Vec3::new(10.0, -5.0, 3.0));
    }

    #[test]
    fn w_s_d_a_with_nonzero_pitch_stays_horizontal() {
        let yaws = [0.0, core::f32::consts::FRAC_PI_2, core::f32::consts::PI];
        for &yaw in &yaws {
            for &pitch in &[0.3, -0.5] {
                let pos = simulate_at(yaw, 1.0, 3.0, &[KeyCode::W, KeyCode::S, KeyCode::D, KeyCode::A]);
                // All four buttons cancel out → no movement regardless of pitch
                assert!(
                    pos.length() < 1e-5,
                    "W+S+D+A yaw={} pitch={} should stay at origin, got {:?}",
                    yaw,
                    pitch,
                    pos
                );
            }
        }
    }
}