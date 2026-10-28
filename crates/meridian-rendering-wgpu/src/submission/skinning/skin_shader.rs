//! The compute kernel that poses a skinned or morphed mesh.

/// Reads a mesh's base vertices (position, normal, UV: eight floats), the
/// four joints and weights of each vertex, a palette of joint matrices, the
/// morph offsets and a control block, and writes the posed vertices in the
/// same layout. The control block is four `u32` (targets, vertices, whether
/// to skin, unused) followed by one `f32` weight per target. Morphing comes
/// first, then skinning; a joint index past the palette uses the last matrix.
pub(crate) const SKIN_SHADER_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> source: array<f32>;
@group(0) @binding(1) var<storage, read> joints: array<vec4<u32>>;
@group(0) @binding(2) var<storage, read> weights: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> palette: array<mat4x4<f32>>;
@group(0) @binding(4) var<storage, read_write> posed: array<f32>;
@group(0) @binding(5) var<storage, read> offsets: array<f32>;
@group(0) @binding(6) var<storage, read> control: array<f32>;

@compute @workgroup_size(64)
fn skin(@builtin(global_invocation_id) id: vec3<u32>) {
    let vertex = id.x;
    let targets = bitcast<u32>(control[0]);
    let count = bitcast<u32>(control[1]);
    if (vertex >= count) {
        return;
    }
    let base = vertex * 8u;
    var base_position = vec3<f32>(source[base], source[base + 1u], source[base + 2u]);
    var base_normal = vec3<f32>(source[base + 3u], source[base + 4u], source[base + 5u]);
    for (var t = 0u; t < targets; t = t + 1u) {
        let weight = control[4u + t];
        let at = (t * count + vertex) * 6u;
        base_position += weight * vec3<f32>(offsets[at], offsets[at + 1u], offsets[at + 2u]);
        base_normal += weight * vec3<f32>(offsets[at + 3u], offsets[at + 4u], offsets[at + 5u]);
    }
    var moved = base_position;
    var turned = base_normal;
    if (bitcast<u32>(control[2]) != 0u) {
        let last = arrayLength(&palette) - 1u;
        let j = joints[vertex];
        let w = weights[vertex];
        let blend = palette[min(j.x, last)] * w.x
            + palette[min(j.y, last)] * w.y
            + palette[min(j.z, last)] * w.z
            + palette[min(j.w, last)] * w.w;
        moved = (blend * vec4<f32>(base_position, 1.0)).xyz;
        turned = (blend * vec4<f32>(base_normal, 0.0)).xyz;
    }
    let length = length(turned);
    if (length > 0.000001) {
        turned = turned / length;
    } else {
        turned = vec3<f32>(source[base + 3u], source[base + 4u], source[base + 5u]);
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
