//! Several sources read as one, the later ones over the earlier.

use std::sync::Arc;

use crate::{AssetListing, AssetPath, ByteSource, SourceError};

/// A stack of listable sources, lowest priority first. A read asks the top
/// source, then the next below it, and so on: the first source that has
/// the path answers, so a patch pack or a loose directory placed over a base
/// pack replaces just the assets it holds. Only *not found* falls through: a
/// source that has the asset but cannot give it (a damaged pack, an I/O
/// error) is that asset's answer, so damage is never hidden by an older copy
/// below it. [`paths`](AssetListing::paths) is the union.
#[derive(Clone)]
pub struct OverlaySource {
    layers: Vec<Arc<dyn AssetListing>>,
}

impl OverlaySource {
    /// `layers`, lowest priority first (the last one wins).
    #[must_use]
    pub fn new(layers: Vec<Arc<dyn AssetListing>>) -> Self {
        Self { layers }
    }

    /// How many sources are stacked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Whether nothing is stacked (every read is not found).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

impl ByteSource for OverlaySource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        for layer in self.layers.iter().rev() {
            match layer.read(path) {
                Err(SourceError::NotFound(_)) => {}
                answer => return answer,
            }
        }
        Err(SourceError::NotFound(path.clone()))
    }
}

impl AssetListing for OverlaySource {
    fn paths(&self) -> Result<Vec<AssetPath>, SourceError> {
        let mut texts = Vec::new();
        for layer in &self.layers {
            texts.extend(
                layer
                    .paths()?
                    .into_iter()
                    .map(|path| path.as_str().to_owned()),
            );
        }
        texts.sort();
        texts.dedup();
        Ok(texts.into_iter().map(AssetPath::new).collect())
    }
}

impl std::fmt::Debug for OverlaySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlaySource")
            .field("layers", &self.layers.len())
            .finish()
    }
}
