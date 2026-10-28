//! Vertex layouts of the mesh buffer and the per-instance buffer.
//!
//! Every pass reads two buffers: slot 0 holds a mesh's vertices once, slot 1
//! one record per drawn instance. A pass declares only the attributes its
//! shader reads; the strides stay those of the full records.

use crate::{VertexAttributeDesc, VertexFormat, VertexLayout, VertexStep};

/// Bytes of one mesh vertex: position, normal, UV.
pub(crate) const MESH_VERTEX_BYTES: u64 = 32;

/// Bytes of one instance record: model matrix, colour, emissive, surface (roughness, metallic).
pub(crate) const INSTANCE_BYTES: u64 = 100;

/// `@location` of the first of the four model matrix columns.
const MODEL_LOCATION: u32 = 3;
const COLOR_LOCATION: u32 = 7;
const EMISSIVE_LOCATION: u32 = 8;
const SURFACE_LOCATION: u32 = 9;

fn attribute(location: u32, format: VertexFormat, offset: u64) -> VertexAttributeDesc {
    VertexAttributeDesc {
        location,
        format,
        offset,
    }
}

/// The four columns of the model matrix, `@location(3..=6)`.
fn model_attributes() -> Vec<VertexAttributeDesc> {
    (0..4)
        .map(|column| {
            attribute(
                MODEL_LOCATION + column,
                VertexFormat::Float32x4,
                u64::from(column) * 16,
            )
        })
        .collect()
}

fn mesh_layout(attributes: Vec<VertexAttributeDesc>) -> VertexLayout {
    VertexLayout {
        stride: MESH_VERTEX_BYTES,
        attributes,
        step: VertexStep::Vertex,
    }
}

fn instance_layout(attributes: Vec<VertexAttributeDesc>) -> VertexLayout {
    VertexLayout {
        stride: INSTANCE_BYTES,
        attributes,
        step: VertexStep::Instance,
    }
}

/// The main passes' mesh buffer: position, normal, UV.
pub(crate) fn mesh_vertex_layout() -> VertexLayout {
    mesh_layout(vec![
        attribute(0, VertexFormat::Float32x3, 0),
        attribute(1, VertexFormat::Float32x3, 12),
        attribute(2, VertexFormat::Float32x2, 24),
    ])
}

/// The main passes' instance buffer: model matrix, colour (or tint),
/// emissive, surface (roughness, metallic).
pub(crate) fn instance_vertex_layout() -> VertexLayout {
    let mut attributes = model_attributes();
    attributes.push(attribute(COLOR_LOCATION, VertexFormat::Float32x4, 64));
    attributes.push(attribute(EMISSIVE_LOCATION, VertexFormat::Float32x3, 80));
    attributes.push(attribute(SURFACE_LOCATION, VertexFormat::Float32x2, 92));
    instance_layout(attributes)
}

/// The depth and emissive passes read only a mesh's position.
pub(crate) fn position_only_mesh_layout() -> VertexLayout {
    mesh_layout(vec![attribute(0, VertexFormat::Float32x3, 0)])
}

/// The shadow depth pass reads only the model matrix of an instance.
pub(crate) fn shadow_instance_layout() -> VertexLayout {
    instance_layout(model_attributes())
}

/// The emissive pass reads the model matrix and the emissive colour.
pub(crate) fn emissive_instance_layout() -> VertexLayout {
    let mut attributes = model_attributes();
    attributes.push(attribute(EMISSIVE_LOCATION, VertexFormat::Float32x3, 80));
    instance_layout(attributes)
}
