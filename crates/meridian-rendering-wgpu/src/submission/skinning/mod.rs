//! Skinning on the GPU (ADR 047): a compute pass turns a skinned mesh's bind
//! pose and a frame's joint matrices into vertices the ordinary passes draw.

mod gpu_skin;
mod skin_job;
mod skin_pipeline;
mod skin_shader;
mod skin_slot;

pub(crate) use gpu_skin::GpuSkin;
pub(crate) use skin_job::SkinJob;
pub(crate) use skin_pipeline::SkinPipeline;
