//! Tests of this module.

use super::*;

/// `BloomPass::new`/`apply` need a real windowed `Surface`/GPU device
/// (the same convention as `SceneRenderer` — see `submission.rs`'s
/// own tests module doc comment); this test covers the one piece
/// that's pure CPU math: the blur uniform's byte layout, which the
/// WGSL side reads as `vec2<f32> texel_size, vec2<f32> direction`.
#[test]
fn blur_uniform_bytes_encode_texel_size_and_direction() {
    let horizontal = blur_uniform_bytes(100, 200, BlurDirection::Horizontal);
    assert_eq!(
        f32::from_le_bytes(horizontal[0..4].try_into().unwrap()),
        0.01
    );
    assert_eq!(
        f32::from_le_bytes(horizontal[4..8].try_into().unwrap()),
        0.005
    );
    assert_eq!(
        f32::from_le_bytes(horizontal[8..12].try_into().unwrap()),
        1.0
    );
    assert_eq!(
        f32::from_le_bytes(horizontal[12..16].try_into().unwrap()),
        0.0
    );

    let vertical = blur_uniform_bytes(100, 200, BlurDirection::Vertical);
    assert_eq!(f32::from_le_bytes(vertical[8..12].try_into().unwrap()), 0.0);
    assert_eq!(
        f32::from_le_bytes(vertical[12..16].try_into().unwrap()),
        1.0
    );
}

#[test]
fn the_mapped_extraction_shader_reads_the_uv_the_factor_and_the_map() {
    let source = crate::submission::EMISSIVE_MAPPED_EXTRACT_SHADER_WGSL;
    for needle in [
        "@location(2) uv",
        "@location(11) mapped_emissive",
        "@binding(1)",
        "@binding(2)",
        "textureSample(emissive_tex",
    ] {
        assert!(source.contains(needle), "missing {needle}");
    }
}
