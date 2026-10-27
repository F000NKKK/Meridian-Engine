//! Wavefront OBJ text to a mesh.

use crate::{DecodeError, Decoder, MeshAsset};

/// Decodes a minimal Wavefront OBJ: `v x y z` vertex lines and `f a b c`
/// triangle face lines (1-indexed; `a/b/c` texture/normal index suffixes
/// are recognized and ignored). No normals, UVs, materials, groups, or
/// non-triangular faces.
#[derive(Debug, Default)]
pub struct ObjDecoder;

impl Decoder<MeshAsset> for ObjDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<MeshAsset, DecodeError> {
        let text =
            core::str::from_utf8(bytes).map_err(|_| DecodeError::Malformed("not valid UTF-8"))?;
        let mut positions = Vec::new();
        let mut indices = Vec::new();

        for line in text.lines() {
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => positions.push(parse_vertex(fields)?),
                Some("f") => indices.extend(parse_face(fields)?),
                _ => {} // comments, normals, UVs, groups, etc. — ignored
            }
        }

        Ok(MeshAsset { positions, indices })
    }
}

/// The three coordinates of a `v x y z` line.
fn parse_vertex<'a>(fields: impl Iterator<Item = &'a str>) -> Result<[f32; 3], DecodeError> {
    let coords: Vec<f32> = fields
        .map(|field| {
            field
                .parse::<f32>()
                .map_err(|_| DecodeError::Malformed("non-numeric vertex coordinate"))
        })
        .collect::<Result<_, _>>()?;
    match coords[..] {
        [x, y, z] => Ok([x, y, z]),
        _ => Err(DecodeError::Malformed(
            "vertex line without exactly 3 coordinates",
        )),
    }
}

/// The zero-based vertex indices of an `f a b c` line.
fn parse_face<'a>(fields: impl Iterator<Item = &'a str>) -> Result<Vec<u32>, DecodeError> {
    let indices: Vec<u32> = fields
        .map(|field| {
            let vertex_index = field.split('/').next().unwrap_or(field);
            vertex_index
                .parse::<i64>()
                .map_err(|_| DecodeError::Malformed("non-numeric face index"))
                .and_then(|one_indexed| {
                    u32::try_from(one_indexed - 1)
                        .map_err(|_| DecodeError::Malformed("face index out of range"))
                })
        })
        .collect::<Result<_, _>>()?;
    if indices.len() != 3 {
        return Err(DecodeError::Unsupported("non-triangular face"));
    }
    Ok(indices)
}
