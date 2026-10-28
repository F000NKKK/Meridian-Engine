//! The compute kernel that poses a skinned mesh.

/// Reads a mesh's bind-pose vertices (position, normal, UV: eight floats),
/// the four joints and weights of each vertex and a palette of joint
/// matrices, and writes the posed vertices in the same layout. A joint index
/// past the palette uses the last matrix.
pub(crate) const SKIN_SHADER_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> source: array<f32>;
@group(0) @binding(1) var<storage, read> joints: array<vec4<u32>>;
@group(0) @binding(2) var<storage, read> weights: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> palette: array<mat4x4<f32>>;
@group(0) @binding(4) var<storage, read_write> posed: array<f32>;

@compute @workgroup_size(64)
fn skin(@builtin(global_invocation_id) id: vec3<u32>) {
    let vertex = id.x;
    if (vertex >= arrayLength(&weights)) {
        return;
    }
    let base = vertex * 8u;
    let position = vec4<f32>(source[base], source[base + 1u], source[base + 2u], 1.0);
    let normal = vec4<f32>(source[base + 3u], source[base + 4u], source[base + 5u], 0.0);
    let last = arrayLength(&palette) - 1u;
    let j = joints[vertex];
    let w = weights[vertex];
    let blend = palette[min(j.x, last)] * w.x
        + palette[min(j.y, last)] * w.y
        + palette[min(j.z, last)] * w.z
        + palette[min(j.w, last)] * w.w;
    let moved = blend * position;
    var turned = (blend * normal).xyz;
    let length = length(turned);
    if (length > 0.000001) {
        turned = turned / length;
    } else {
        turned = normal.xyz;
    }
    posed[base] = moved.x;
    posed[base + 1u] = moved.y;
    posed[base + 2u] = moved.z;
    posed[base + 3u] = turned.x;
    posed[base + 4u] = turned.y;
    posed[base + 5u] = turned.z;
    posed[base + 6u] = source[base + 6u];
    posed[base + 7u] = source[base + 7u];
}
";
