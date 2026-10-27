//! A GPU texture and its default view.

/// A GPU texture resource, with its default view already created.
#[derive(Debug)]
pub struct Texture {
    pub(crate) raw: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
