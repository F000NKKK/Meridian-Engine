//! The vertex stage every full-screen post-process pass shares.

/// A full-screen-triangle vertex shader shared by both blur directions
/// and the composite and tone mapping passes — see `rendering-wgpu`'s
/// `Device::create_fullscreen_pipeline` doc comment for why no vertex
/// buffer is needed.
pub(crate) const FULLSCREEN_VERTEX_WGSL: &str = r#"
struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: VsOut;
    out.clip_position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}
"#;
