//! The upload, dispatch and read-back sequence both flavors' soft-body
//! kernels share.

use crate::gpu_backend_missing::{GpuBackendMissing, require_gpu};
use meridian_compute_app::ComputeContext;
use meridian_gpu_wgpu::{BufferUsage, ComputePipeline};

use super::scratch::SoftBodyScratch;
use super::step_inputs::StepInputs;

/// Threads per workgroup of the soft-body step shaders.
const WORKGROUP_SIZE: u32 = 64;

/// Uploads `inputs`, dispatches one thread per particle and reads the new
/// positions and velocities back (in that order). `kernel` names the calling operation in
/// [`GpuBackendMissing`] when `context` has no GPU backend.
pub(super) async fn run(
    pipeline: &ComputePipeline,
    context: &ComputeContext,
    inputs: &StepInputs,
    scratch: &mut SoftBodyScratch,
    kernel: &'static str,
) -> Result<(Vec<u8>, Vec<u8>), GpuBackendMissing> {
    let gpu = require_gpu(context, kernel)?;
    let storage = BufferUsage::Storage;
    let uploads = [
        (&inputs.params, BufferUsage::Uniform),
        (&inputs.positions, storage),
        (&inputs.velocities, storage),
        (&inputs.inverse_masses, storage),
        (&inputs.edge_offsets, storage),
        (&inputs.edge_neighbor, storage),
        (&inputs.edge_params, storage),
    ];
    for (slot, (bytes, usage)) in scratch.slots.iter_mut().zip(uploads) {
        slot.upload(gpu, bytes, usage);
    }
    let [.., positions_out, velocities_out] = &mut scratch.slots;
    positions_out.reserve(gpu, inputs.positions.len(), storage);
    velocities_out.reserve(gpu, inputs.velocities.len(), storage);

    let device = gpu.gpu_driver_device();
    let generations = scratch.generations();
    let slots = &scratch.slots;
    let bind_group = scratch.bind_group.reuse_or_build(&generations, || {
        let buffers = slots
            .each_ref()
            .map(|slot| slot.buffer().expect("every slot was filled"));
        device.create_bind_group(&pipeline.bind_group_layout(), &buffers)
    });
    let mut commands = device.create_command_buffer();
    commands.dispatch_compute_with_bind_group(
        pipeline,
        bind_group,
        (inputs.particle_count as u32)
            .div_ceil(WORKGROUP_SIZE)
            .max(1),
    );
    commands.submit();

    // The output buffers may be larger than this step's data; cut the
    // read-back to what the step produced.
    let [.., positions_slot, velocities_slot] = &scratch.slots;
    let mut positions = gpu
        .read_buffer(positions_slot.buffer().expect("filled"))
        .await;
    positions.truncate(inputs.positions.len());
    let mut velocities = gpu
        .read_buffer(velocities_slot.buffer().expect("filled"))
        .await;
    velocities.truncate(inputs.velocities.len());
    Ok((positions, velocities))
}
