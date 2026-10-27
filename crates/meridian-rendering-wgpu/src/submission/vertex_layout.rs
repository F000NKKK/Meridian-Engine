//! Vertex layouts for the colored, textured, emissive and shadow passes.

use crate::{VertexAttributeDesc, VertexFormat, VertexLayout};

/// [`UNLIT_SHADER_WGSL`](super::UNLIT_SHADER_WGSL)/[`lit_shader_wgsl`](super::lit_shader_wgsl)'s shared vertex layout:
/// position, normal, color, emissive — tightly packed.
pub(crate) fn colored_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 52, // 3 + 3 + 4 + 3 = 13 floats
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
            VertexAttributeDesc {
                location: 2,
                format: VertexFormat::Float32x4,
                offset: 24,
            },
            VertexAttributeDesc {
                location: 3,
                format: VertexFormat::Float32x3,
                offset: 40,
            },
        ],
    }
}

/// [`TEXTURED_SHADER_WGSL`](super::TEXTURED_SHADER_WGSL)/[`lit_textured_shader_wgsl`](super::lit_textured_shader_wgsl)'s shared vertex
/// layout: position, normal, uv, tint, emissive — tightly packed.
pub(crate) fn textured_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 60, // 3 + 3 + 2 + 4 + 3 = 15 floats
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
            VertexAttributeDesc {
                location: 2,
                format: VertexFormat::Float32x2,
                offset: 24,
            },
            VertexAttributeDesc {
                location: 3,
                format: VertexFormat::Float32x4,
                offset: 32,
            },
            VertexAttributeDesc {
                location: 4,
                format: VertexFormat::Float32x3,
                offset: 48,
            },
        ],
    }
}

/// Reinterprets [`colored_vertex_layout`]'s buffer bytes as just
/// position + emissive (same stride, two attributes) — what
/// [`EMISSIVE_EXTRACT_SHADER_WGSL`](super::EMISSIVE_EXTRACT_SHADER_WGSL) reads for a colored-family draw.
pub(crate) fn emissive_from_colored_layout() -> VertexLayout {
    VertexLayout {
        stride: 52,
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 40,
            },
        ],
    }
}

/// [`emissive_from_colored_layout`]'s textured-family counterpart.
pub(crate) fn emissive_from_textured_layout() -> VertexLayout {
    VertexLayout {
        stride: 60,
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 48,
            },
        ],
    }
}

/// Position-only reinterpretation of [`colored_vertex_layout`]'s buffer
/// bytes — what the shadow depth pass reads for a colored-family draw
/// (see [`SHADOW_DEPTH_SHADER_WGSL`](super::SHADOW_DEPTH_SHADER_WGSL)): the depth pass only needs
/// clip-space position, so normal/color/emissive are never touched,
/// even though they're still physically present in the same buffer.
pub(crate) fn shadow_from_colored_layout() -> VertexLayout {
    VertexLayout {
        stride: 52,
        attributes: vec![VertexAttributeDesc {
            location: 0,
            format: VertexFormat::Float32x3,
            offset: 0,
        }],
    }
}

/// `shadow_from_colored_layout`'s textured-family counterpart.
pub(crate) fn shadow_from_textured_layout() -> VertexLayout {
    VertexLayout {
        stride: 60,
        attributes: vec![VertexAttributeDesc {
            location: 0,
            format: VertexFormat::Float32x3,
            offset: 0,
        }],
    }
}
