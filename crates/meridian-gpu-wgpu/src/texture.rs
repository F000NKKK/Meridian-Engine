//! A GPU texture and its default view.

/// A GPU texture resource, with its default view already created.
#[derive(Debug)]
pub struct Texture {
    pub(crate) raw: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
}

impl Texture {
    /// The default view of the texture, created together with it.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
