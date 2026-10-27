//! Wavefront OBJ text to a mesh.

use crate::DecodeError;
use crate::Decoder;
use crate::MeshAsset;

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
            let line = line.trim();
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => {
                    let coords: Vec<f32> = fields
                        .map(|f| {
                            f.parse::<f32>().map_err(|_| {
                                DecodeError::Malformed("non-numeric vertex coordinate")
                            })
                        })
                        .collect::<Result<_, _>>()?;
                    if coords.len() != 3 {
                        return Err(DecodeError::Malformed(
                            "vertex line without exactly 3 coordinates",
                        ));
                    }
                    positions.push([coords[0], coords[1], coords[2]]);
                }
                Some("f") => {
                    let idx: Vec<u32> = fields
                        .map(|f| {
                            let vertex_index = f.split('/').next().unwrap_or(f);
                            vertex_index
                                .parse::<i64>()
                                .map_err(|_| DecodeError::Malformed("non-numeric face index"))
                        })
                        .collect::<Result<Vec<i64>, _>>()?
                        .into_iter()
                        .map(|one_indexed| (one_indexed - 1) as u32)
                        .collect();
                    if idx.len() != 3 {
                        return Err(DecodeError::Unsupported("non-triangular face"));
                    }
                    indices.extend(idx);
                }
                _ => {} // comments, normals, UVs, groups, etc. — ignored
            }
        }

        Ok(MeshAsset { positions, indices })
    }
}
