//! [`Layers`]: where the application's assets are, in overlay order.

use std::path::PathBuf;

use super::Layer;

/// The startup choice of where assets come from: one [`Layer`] or a stack of
/// them, lowest priority first. Never empty. Every layer reads exactly the
/// same way through [`Mounted`](super::Mounted), so what is chosen here
/// changes nothing for the code that loads.
///
/// ```
/// # use meridian_sdk::content::{Layer, Layers};
/// // A base pack, patched by a second one, with loose files on top.
/// let layers = Layers::pack("base.mpak").over(Layer::Pack("patch.mpak".into()))
///     .over(Layer::Directory("dev-assets".into()));
/// assert_eq!(layers.layers().len(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layers(Vec<Layer>);

impl Layers {
    /// A single layer.
    #[must_use]
    pub fn new(base: Layer) -> Self {
        Layers(vec![base])
    }

    /// Loose files under `directory`.
    #[must_use]
    pub fn directory(directory: impl Into<PathBuf>) -> Self {
        Self::new(Layer::Directory(directory.into()))
    }

    /// One pack.
    #[must_use]
    pub fn pack(pack: impl Into<PathBuf>) -> Self {
        Self::new(Layer::Pack(pack.into()))
    }

    /// `layer` over the ones already here: it is asked first.
    #[must_use]
    pub fn over(mut self, layer: Layer) -> Self {
        self.0.push(layer);
        self
    }

    /// The layers, lowest priority first.
    #[must_use]
    pub fn layers(&self) -> &[Layer] {
        &self.0
    }
}

impl From<Layer> for Layers {
    fn from(layer: Layer) -> Self {
        Layers::new(layer)
    }
}
