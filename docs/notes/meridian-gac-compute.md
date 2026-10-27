# `meridian-gac-wgpu` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `fixed_wgsl`

**Scope: `+`/`-`/`*`/`/`/`sqrt`/`sin_cos`/`atan2`.** The CORDIC
`sin_cos`/`atan2` port (see [`CORDIC_LIB_WGSL`]) needs no 64-bit
emulation, unlike the arithmetic above — CORDIC is only shifts and
adds on a single `i32`, and WGSL's `>>` on `i32` is already an
arithmetic (sign-extending) shift, exactly matching
`meridian_numeric::fixed`'s `Fixed(x.0 >> i)` — so it's a near-
literal translation of `cordic_rotate`/`cordic_vector_angle`, not a
second emulation layer. The `atan` table and gain/pi constants are
precomputed on the Rust side (`Fixed::from_num`'s exact
round-half-away-from-zero rounding, replicated by hand — see
[`CORDIC_LIB_WGSL`]'s constants) and embedded as WGSL literals rather
than recomputed in-shader, since they're fixed at compile time either
way.

[`FixedArithmeticKernels`] compiles the four binary-op compute
pipelines once against a [`meridian_compute_app::ComputeContext`]'s
GPU backend (see `compute-wgpu::gpu_context`)
and [`FixedArithmeticKernels::dispatch`] runs one operation over a
batch of operand pairs, returning one [`Fixed`] result per pair — the
real, reusable API this phase's work is for, not just a
proof-of-concept hidden in a test.
