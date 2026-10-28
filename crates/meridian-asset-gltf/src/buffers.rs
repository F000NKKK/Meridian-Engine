//! The bytes behind a document's buffers.

use gltf::Document;
use gltf::buffer::Source;

use crate::GltfError;
use crate::resources::{Resources, resolve};

/// The data of every buffer, indexed like `document.buffers()`. A buffer
/// is the GLB's `BIN` chunk, a `data:` URI or a supplied external file.
pub(crate) fn load(
    document: &Document,
    blob: Option<Vec<u8>>,
    resources: &Resources,
) -> Result<Vec<Vec<u8>>, GltfError> {
    let mut blob = blob;
    document
        .buffers()
        .map(|buffer| match buffer.source() {
            Source::Bin => blob.take().ok_or(GltfError::Malformed(
                "buffer expects a BIN chunk".to_owned(),
            )),
            Source::Uri(uri) => resolve(uri, resources),
        })
        .collect()
}
