//! The bytes behind a document's buffers.

use gltf::Document;
use gltf::buffer::Source;

use crate::{GltfError, data_uri};

/// The data of every buffer, indexed like `document.buffers()`. A buffer
/// is the GLB's `BIN` chunk or a `data:` URI; anything else is external.
pub(crate) fn load(document: &Document, blob: Option<Vec<u8>>) -> Result<Vec<Vec<u8>>, GltfError> {
    let mut blob = blob;
    document
        .buffers()
        .map(|buffer| match buffer.source() {
            Source::Bin => blob.take().ok_or(GltfError::Malformed(
                "buffer expects a BIN chunk".to_owned(),
            )),
            Source::Uri(uri) => data_uri::decode(uri),
        })
        .collect()
}
