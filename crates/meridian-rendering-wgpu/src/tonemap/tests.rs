//! Tests of the tone mapping pass.

use meridian_rendering_domain::{TonemapConfig, TonemapOperator};

use super::{tonemap_shader_wgsl, uniform_bytes};

#[test]
fn uniform_bytes_carry_exposure_then_operator_index() {
    let config = TonemapConfig {
        exposure: 2.5,
        operator: TonemapOperator::Reinhard,
    };
    let bytes = uniform_bytes(config);
    assert_eq!(&bytes[0..4], &2.5f32.to_le_bytes());
    assert_eq!(&bytes[4..8], &1.0f32.to_le_bytes());
}

#[test]
fn the_shader_has_the_fullscreen_stage_and_a_fragment_entry() {
    let source = tonemap_shader_wgsl();
    assert!(source.contains("fn vs_main"));
    assert!(source.contains("fn fs_main"));
}
