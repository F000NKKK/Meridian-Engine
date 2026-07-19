//! Low-level GPU abstraction (device, command queues, buffers, textures, shaders, pipelines, synchronization). Knows nothing about scenes or materials.

/// A GPU device.
#[derive(Debug)]
pub struct Device;

/// A recorded, submittable sequence of GPU commands.
#[derive(Debug, Default)]
pub struct CommandBuffer;

/// A GPU-visible buffer (vertex/index/uniform/storage).
#[derive(Debug, Clone, Copy, Default)]
pub struct Buffer {
    pub byte_len: usize,
}

/// A GPU texture resource.
#[derive(Debug, Clone, Copy, Default)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
}

/// A compiled shader module.
#[derive(Debug)]
pub struct Shader;

/// A configured GPU pipeline (shaders + fixed-function state).
#[derive(Debug)]
pub struct Pipeline;
