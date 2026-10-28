//! Skinning and morphing on the GPU (ADR 047): a compute pass turns a
//! mesh's bind pose, a frame's joint matrices and morph weights into
//! vertices the ordinary passes draw.

mod gpu_morph;
mod gpu_skin;
mod mesh_pose;
mod skin_job;
mod skin_pipeline;
mod skin_shader;
mod skin_slot;

pub(crate) use gpu_morph::GpuMorph;
pub(crate) use gpu_skin::GpuSkin;
pub(crate) use mesh_pose::MeshPose;
pub(crate) use skin_job::SkinJob;
pub(crate) use skin_pipeline::SkinPipeline;

#[cfg(all(test, feature = "hw-tests"))]
mod tests;
