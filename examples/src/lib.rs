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

    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        )
        .normalize()
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

        let forward = self.forward();
        let right = Vec3::Y.cross(forward).normalize();
        let mut movement = Vec3::ZERO;
        if input.is_key_down(KeyCode::W) {
            movement = movement + forward;
        }
        if input.is_key_down(KeyCode::S) {
            movement = movement - forward;
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
        let yaw_rotor = Rotor::from_axis_angle(Vec3::Y, self.yaw);
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
