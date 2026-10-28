//! One glTF primitive as a world-space model part.

use gltf::Primitive;
use gltf::mesh::{Mode, Semantic};
use meridian_asset_domain::ModelPart;

use crate::limits::MAX_PRIMITIVE_VERTICES;
use crate::{GltfError, material_reader};

type Matrix = [[f32; 4]; 4];

/// Reads `primitive` in its own space, then places it with `world`.
pub(crate) fn read(
    primitive: &Primitive<'_>,
    world: &Matrix,
    buffers: &[Vec<u8>],
) -> Result<ModelPart, GltfError> {
    let mut part = load(primitive, buffers)?;
    place(&mut part, world);
    Ok(part)
}

/// The primitive's attributes and indices, validated, untransformed.
fn load(primitive: &Primitive<'_>, buffers: &[Vec<u8>]) -> Result<ModelPart, GltfError> {
    if primitive.mode() != Mode::Triangles {
        return Err(GltfError::UnsupportedPrimitive(format!(
            "{:?}",
            primitive.mode()
        )));
    }
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    let positions: Vec<[f32; 3]> = reader
        .read_positions()
        .ok_or(GltfError::OutOfBounds("positions"))?
        .collect();
    if positions.len() > MAX_PRIMITIVE_VERTICES {
        return Err(GltfError::TooManyVertices(positions.len()));
    }
    let normals: Vec<[f32; 3]> = declared(
        primitive.get(&Semantic::Normals).is_some(),
        reader.read_normals().map(Iterator::collect),
        "normals",
    )?;
    let uvs: Vec<[f32; 2]> = declared(
        primitive.get(&Semantic::TexCoords(0)).is_some(),
        reader
            .read_tex_coords(0)
            .map(|uvs| uvs.into_f32().collect()),
        "texture coordinates",
    )?;
    let indices: Vec<u32> = if primitive.indices().is_some() {
        let read = reader.read_indices().map(|i| i.into_u32().collect());
        declared(true, read, "indices")?
    } else {
        (0..u32::try_from(positions.len()).map_err(|_| GltfError::BadIndices)?).collect()
    };
    check(&positions, &normals, &uvs, &indices)?;

    Ok(ModelPart {
        positions,
        normals,
        uvs,
        indices,
        material: material_reader::read(&primitive.material()),
    })
}

/// Applies `world` to the positions and normals. A transform that mirrors
/// the model (negative determinant) also reverses the winding so front
/// faces stay front faces.
fn place(part: &mut ModelPart, world: &Matrix) {
    for position in &mut part.positions {
        *position = transform_point(world, *position);
    }
    for normal in &mut part.normals {
        *normal = transform_normal(world, *normal);
    }
    if determinant(world) < 0.0 {
        for triangle in part.indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }
}

/// Per-vertex attributes line up and every index lands on a vertex.
fn check(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u32],
) -> Result<(), GltfError> {
    let aligned = |len: usize| len == 0 || len == positions.len();
    if !aligned(normals.len()) || !aligned(uvs.len()) {
        return Err(GltfError::OutOfBounds("attribute count"));
    }
    let in_range = indices.iter().all(|&i| (i as usize) < positions.len());
    if !indices.len().is_multiple_of(3) || !in_range {
        return Err(GltfError::BadIndices);
    }
    Ok(())
}

fn transform_point(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row])
}

/// The inverse-transpose of the upper 3x3 (its cofactor matrix), so normals
/// stay perpendicular under non-uniform scale; renormalised afterwards.
fn transform_normal(m: &Matrix, n: [f32; 3]) -> [f32; 3] {
    let at = |row: usize, col: usize| m[col][row];
    let cofactor = |i: usize, j: usize| {
        let (i1, i2, j1, j2) = ((i + 1) % 3, (i + 2) % 3, (j + 1) % 3, (j + 2) % 3);
        at(i1, j1) * at(i2, j2) - at(i1, j2) * at(i2, j1)
    };
    let out: [f32; 3] = std::array::from_fn(|i| (0..3).map(|j| cofactor(i, j) * n[j]).sum());
    let length = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
    if length > f32::EPSILON {
        out.map(|v| v / length)
    } else {
        n
    }
}

fn determinant(m: &Matrix) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
        - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
        + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2])
}

/// What the reader returned for an attribute: empty when the primitive does
/// not declare it, an error when it declares one the buffers cannot hold.
fn declared<T>(
    present: bool,
    read: Option<Vec<T>>,
    what: &'static str,
) -> Result<Vec<T>, GltfError> {
    match (present, read) {
        (_, Some(values)) => Ok(values),
        (false, None) => Ok(Vec::new()),
        (true, None) => Err(GltfError::OutOfBounds(what)),
    }
}
