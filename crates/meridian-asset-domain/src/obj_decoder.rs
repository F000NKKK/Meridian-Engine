//! Wavefront OBJ text to a mesh with positions, texture coordinates and normals.

use std::collections::HashMap;

use crate::{DecodeError, Decoder, MeshAsset};

/// Decodes triangulated Wavefront OBJ geometry. Distinct position/UV/normal
/// combinations become distinct vertices, as required by GPU vertex buffers.
#[derive(Debug, Default)]
pub struct ObjDecoder;

impl Decoder<MeshAsset> for ObjDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<MeshAsset, DecodeError> {
        let text =
            core::str::from_utf8(bytes).map_err(|_| DecodeError::Malformed("not valid UTF-8"))?;
        let mut source = Source::default();
        let mut output = Output::default();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => source.positions.push(parse_vertex(fields)?),
                Some("vt") => source.uvs.push(parse_uv(fields)?),
                Some("vn") => source.normals.push(parse_vertex(fields)?),
                Some("f") => output.face(fields, &source)?,
                _ => {}
            }
        }
        Ok(output.finish())
    }
}

#[derive(Default)]
struct Source {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FaceVertex {
    position: usize,
    uv: Option<usize>,
    normal: Option<usize>,
}

#[derive(Default)]
struct Output {
    mesh: MeshAsset,
    vertices: HashMap<FaceVertex, u32>,
    complete_uvs: bool,
    complete_normals: bool,
}

impl Output {
    fn face<'a>(
        &mut self,
        fields: impl Iterator<Item = &'a str>,
        source: &Source,
    ) -> Result<(), DecodeError> {
        let fields: Vec<_> = fields.collect();
        if fields.len() != 3 {
            return Err(DecodeError::Unsupported("non-triangular face"));
        }
        if self.mesh.indices.is_empty() {
            self.complete_uvs = true;
            self.complete_normals = true;
        }
        for field in fields {
            let vertex = parse_face_vertex(field, source)?;
            let index = self.vertex(vertex, source);
            self.mesh.indices.push(index);
        }
        Ok(())
    }

    fn vertex(&mut self, vertex: FaceVertex, source: &Source) -> u32 {
        if let Some(index) = self.vertices.get(&vertex) {
            return *index;
        }
        let index = self.mesh.positions.len() as u32;
        self.mesh.positions.push(source.positions[vertex.position]);
        self.mesh
            .normals
            .push(vertex.normal.map_or([0.0; 3], |at| source.normals[at]));
        self.mesh
            .uvs
            .push(vertex.uv.map_or([0.0; 2], |at| source.uvs[at]));
        self.complete_uvs &= vertex.uv.is_some();
        self.complete_normals &= vertex.normal.is_some();
        self.vertices.insert(vertex, index);
        index
    }

    fn finish(mut self) -> MeshAsset {
        if !self.complete_normals {
            self.mesh.normals.clear();
        }
        if !self.complete_uvs {
            self.mesh.uvs.clear();
        }
        self.mesh
    }
}

fn parse_face_vertex(text: &str, source: &Source) -> Result<FaceVertex, DecodeError> {
    let mut fields = text.split('/');
    let position = parse_index(fields.next().unwrap_or_default(), source.positions.len())?;
    let uv = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(|value| parse_index(value, source.uvs.len()))
        .transpose()?;
    let normal = fields
        .next()
        .filter(|value| !value.is_empty())
        .map(|value| parse_index(value, source.normals.len()))
        .transpose()?;
    Ok(FaceVertex {
        position,
        uv,
        normal,
    })
}

fn parse_index(text: &str, len: usize) -> Result<usize, DecodeError> {
    let index = text
        .parse::<i64>()
        .map_err(|_| DecodeError::Malformed("non-numeric face index"))?;
    let resolved = if index < 0 {
        len as i64 + index
    } else {
        index - 1
    };
    if resolved < 0 || resolved >= len as i64 {
        return Err(DecodeError::Malformed("face index out of range"));
    }
    Ok(resolved as usize)
}

fn parse_vertex<'a>(fields: impl Iterator<Item = &'a str>) -> Result<[f32; 3], DecodeError> {
    parse_array(fields, "vertex line without exactly 3 coordinates")
}

fn parse_uv<'a>(fields: impl Iterator<Item = &'a str>) -> Result<[f32; 2], DecodeError> {
    parse_array(fields, "texture coordinate line without exactly 2 coordinates")
}

fn parse_array<'a, const N: usize>(
    fields: impl Iterator<Item = &'a str>,
    error: &'static str,
) -> Result<[f32; N], DecodeError> {
    let coords: Vec<f32> = fields
        .map(|field| {
            field
                .parse::<f32>()
                .map_err(|_| DecodeError::Malformed("non-numeric vertex coordinate"))
        })
        .collect::<Result<_, _>>()?;
    coords
        .try_into()
        .map_err(|_| DecodeError::Malformed(error))
}
