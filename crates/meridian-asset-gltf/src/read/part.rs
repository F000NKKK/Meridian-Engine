//! One glTF primitive as a world-space model part.

use gltf::Primitive;
use gltf::mesh::{Mode, Semantic};
use meridian_asset_domain::{ModelPart, PartSkin};

use crate::GltfError;
use crate::read::skin::SkinMap;
use crate::read::{material, morph};

type Matrix = [[f32; 4]; 4];

/// Where a primitive goes: placed by its node's `world`, or, when it has a
/// skin whose joints and weights the primitive carries, bound to it.
pub(crate) struct Placement<'a> {
    pub(crate) world: &'a Matrix,
    pub(crate) skin: Option<&'a SkinMap>,
    /// The `(skeleton, joint)` that moves a rigid part animation moves:
    /// the part stays in its node's space, bound to that joint alone.
    pub(crate) motion: Option<(usize, usize)>,
}

/// Reads `primitive` in its own space, then places or skins it.
pub(crate) fn read(
    primitive: &Primitive<'_>,
    placement: &Placement<'_>,
    buffers: &[Vec<u8>],
) -> Result<ModelPart, GltfError> {
    let mut part = load(primitive, buffers)?;
    let skin = match (placement.skin, placement.motion) {
        (Some(map), _) => skin_of(primitive, buffers, map, part.positions.len())?,
        (None, Some((skeleton, joint))) => Some(on_joint(skeleton, joint, part.positions.len())?),
        (None, None) => None,
    };
    if skin.is_some() {
        part.skin = skin;
    } else {
        place(&mut part, placement.world);
    }
    Ok(part)
}

/// A part bound entirely to one joint.
fn on_joint(skeleton: usize, joint: usize, vertices: usize) -> Result<PartSkin, GltfError> {
    let joint = u16::try_from(joint).map_err(|_| GltfError::OutOfBounds("joint index"))?;
    Ok(PartSkin {
        skeleton,
        joints: vec![[joint, 0, 0, 0]; vertices],
        weights: vec![[1.0, 0.0, 0.0, 0.0]; vertices],
    })
}

/// The four joints and weights per vertex, remapped to the skeleton's joint
/// order; `None` when the primitive has no `JOINTS_0`/`WEIGHTS_0`.
fn skin_of(
    primitive: &Primitive<'_>,
    buffers: &[Vec<u8>],
    map: &SkinMap,
    vertices: usize,
) -> Result<Option<PartSkin>, GltfError> {
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    let (Some(joints), Some(weights)) = (reader.read_joints(0), reader.read_weights(0)) else {
        return Ok(None);
    };
    let joints: Vec<[u16; 4]> = joints
        .into_u16()
        .map(|four| remap(four, &map.remap))
        .collect::<Result<_, _>>()?;
    let weights: Vec<[f32; 4]> = weights.into_f32().map(normalised).collect();
    if joints.len() != vertices || weights.len() != vertices {
        return Err(GltfError::OutOfBounds("skin attribute count"));
    }
    Ok(Some(PartSkin {
        skeleton: map.skeleton,
        joints,
        weights,
    }))
}

fn remap(four: [u16; 4], remap: &[usize]) -> Result<[u16; 4], GltfError> {
    let mut out = [0u16; 4];
    for (slot, joint) in out.iter_mut().zip(four) {
        let sorted = remap
            .get(usize::from(joint))
            .ok_or(GltfError::OutOfBounds("joint index"))?;
        *slot = u16::try_from(*sorted).map_err(|_| GltfError::OutOfBounds("joint index"))?;
    }
    Ok(out)
}

/// `weights` scaled to sum to one; a vertex with none follows its first joint.
fn normalised(weights: [f32; 4]) -> [f32; 4] {
    let sum: f32 = weights.iter().sum();
    if sum > f32::EPSILON {
        weights.map(|weight| weight / sum)
    } else {
        [1.0, 0.0, 0.0, 0.0]
    }
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
    let positions_len = positions.len();

    Ok(ModelPart {
        positions,
        normals,
        uvs,
        indices,
        material: material::read(&primitive.material()),
        skin: None,
        morph: morph::read(primitive, buffers, positions_len)?,
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
    for target in part.morph.iter_mut().flat_map(|morph| &mut morph.targets) {
        for offset in &mut target.positions {
            *offset = transform_vector(world, *offset);
        }
        for offset in &mut target.normals {
            *offset = cofactor_transform(world, *offset);
        }
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

/// `v` through the upper 3x3 alone: a direction or offset, which a
/// translation does not move.
fn transform_vector(m: &Matrix, v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| m[0][row] * v[0] + m[1][row] * v[1] + m[2][row] * v[2])
}

/// `n` through the cofactor matrix of the upper 3x3 (the inverse-transpose
/// up to scale), which keeps normals perpendicular under non-uniform scale.
fn cofactor_transform(m: &Matrix, n: [f32; 3]) -> [f32; 3] {
    let at = |row: usize, col: usize| m[col][row];
    let cofactor = |i: usize, j: usize| {
        let (i1, i2, j1, j2) = ((i + 1) % 3, (i + 2) % 3, (j + 1) % 3, (j + 2) % 3);
        at(i1, j1) * at(i2, j2) - at(i1, j2) * at(i2, j1)
    };
    std::array::from_fn(|i| (0..3).map(|j| cofactor(i, j) * n[j]).sum())
}

/// A normal under `m`, renormalised.
fn transform_normal(m: &Matrix, n: [f32; 3]) -> [f32; 3] {
    let out = cofactor_transform(m, n);
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
