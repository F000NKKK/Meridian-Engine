//! The still-encoded images of a model.

use gltf::Document;
use gltf::image::Source;
use meridian_asset_domain::EncodedImage;

use crate::{GltfError, data_uri};

/// Every image of `document`, indexed like `document.images()`, as the
/// bytes of its file. Decoding them is the application's job.
pub(crate) fn read(
    document: &Document,
    buffers: &[Vec<u8>],
) -> Result<Vec<EncodedImage>, GltfError> {
    document
        .images()
        .map(|image| {
            let bytes = match image.source() {
                Source::View { view, .. } => {
                    let buffer = &buffers[view.buffer().index()];
                    let end = view.offset().checked_add(view.length());
                    end.and_then(|end| buffer.get(view.offset()..end))
                        .ok_or(GltfError::OutOfBounds("image buffer view"))?
                        .to_vec()
                }
                Source::Uri { uri, .. } => data_uri::decode(uri)?,
            };
            Ok(EncodedImage { bytes })
        })
        .collect()
}
