//! WGSL dispatch entry points, one per operation.

use super::{CORDIC_LIB_WGSL, FIXED_ARITHMETIC_LIB_WGSL};

/// The four dispatch entry points, sharing [`FIXED_ARITHMETIC_LIB_WGSL`](super::FIXED_ARITHMETIC_LIB_WGSL):
/// each reads one `(a, b)` pair from `operands` and writes one result
/// into `results`, both `array<i32>` (a `Fixed`'s raw Q16.16 bits) —
/// `operands[2*id]`/`operands[2*id+1]` are `a`/`b` for invocation `id`,
/// `results[id]` is that invocation's output. Two buffers (not the
/// single-binding shape `gpu-wgpu`'s own `dispatch_compute` supports)
/// means this crate records its own compute pass rather than reusing
/// that convenience method — see [`FixedArithmeticKernels::dispatch`](super::FixedArithmeticKernels::dispatch).
pub(crate) fn dispatch_entry_points_wgsl() -> String {
    format!(
        r#"
{FIXED_ARITHMETIC_LIB_WGSL}
{CORDIC_LIB_WGSL}

@group(0) @binding(0)
var<storage, read> operands: array<i32>;

@group(0) @binding(1)
var<storage, read_write> results: array<i32>;

@compute @workgroup_size(64)
fn dispatch_add(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_add(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

@compute @workgroup_size(64)
fn dispatch_sub(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_sub(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

@compute @workgroup_size(64)
fn dispatch_mul(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_mul(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

@compute @workgroup_size(64)
fn dispatch_div(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_div(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

// Unary, unlike the four above: one operand per invocation
// (`operands[id.x]`, not a `2*id.x`/`2*id.x+1` pair) — see
// `FixedArithmeticKernels::dispatch_sqrt`, which populates `operands`
// accordingly.
@compute @workgroup_size(64)
fn dispatch_sqrt(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_sqrt(operands[id.x]);
}}

// Binary, like dispatch_add/etc: atan2(a, b) is one result per pair.
@compute @workgroup_size(64)
fn dispatch_atan2(@builtin(global_invocation_id) id: vec3<u32>) {{
    results[id.x] = fixed_atan2(operands[2u * id.x], operands[2u * id.x + 1u]);
}}

// Unary input, binary output: one operand per invocation
// (`operands[id.x]`), two results (`results[2*id.x]` = sin,
// `results[2*id.x+1]` = cos) — see
// `FixedArithmeticKernels::dispatch_sin_cos`, which sizes/unpacks the
// `results` buffer accordingly.
@compute @workgroup_size(64)
fn dispatch_sin_cos(@builtin(global_invocation_id) id: vec3<u32>) {{
    let sc = fixed_sin_cos(operands[id.x]);
    results[2u * id.x] = sc.x;
    results[2u * id.x + 1u] = sc.y;
}}
"#
    )
}
