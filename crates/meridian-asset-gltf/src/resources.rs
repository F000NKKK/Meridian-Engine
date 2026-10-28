//! Files a `.gltf` names next to itself.

use std::collections::HashMap;

use crate::{GltfError, data_uri};

/// The bytes of the external files a model names, by the URI the model uses
/// (`"scene.bin"`, `"textures/wall.png"`). The decoder reads no files; the
/// loader fetches [`GltfDecoder::required_uris`](crate::GltfDecoder::required_uris)
/// and hands the result to [`GltfDecoder::decode_with`](crate::GltfDecoder::decode_with).
pub type Resources = HashMap<String, Vec<u8>>;

/// The bytes behind `uri`: decoded from a `data:` URI, or looked up in
/// `resources`.
pub(crate) fn resolve(uri: &str, resources: &Resources) -> Result<Vec<u8>, GltfError> {
    if uri.starts_with("data:") {
        return data_uri::decode(uri);
    }
    resources
        .get(uri)
        .cloned()
        .ok_or_else(|| GltfError::ExternalResource(uri.to_owned()))
}
