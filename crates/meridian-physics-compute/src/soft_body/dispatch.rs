//! The upload, dispatch and read-back sequence both flavors' soft-body
//! kernels share.

use meridian_compute_driver::GpuComputeDevice;
use meridian_compute_runtime::ComputeContext;
use meridian_gpu_driver::{Buffer, BufferUsage, ComputePipeline};

use super::step_inputs::StepInputs;

/// Threads per workgroup of the soft-body step shaders.
const WORKGROUP_SIZE: u32 = 64;

fn upload(
    context: &ComputeContext,
    bytes: &[u8],
    usage: BufferUsage,
    gpu_missing: &'static str,
) -> Buffer {
    let gpu = context.gpu::<GpuComputeDevice>().expect(gpu_missing);
    let buffer = gpu.allocate_buffer(bytes.len(), usage);
    gpu.write_buffer(&buffer, bytes);
    buffer
}

/// Uploads `inputs`, dispatches one thread per particle and reads the new
/// positions and velocities back (in that order). `gpu_missing` is the panic message when
/// `context` has no GPU backend.
pub(super) async fn run(
    pipeline: &ComputePipeline,
    context: &ComputeContext,
    inputs: &StepInputs,
    gpu_missing: &'static str,
) -> (Vec<u8>, Vec<u8>) {
    let gpu = context.gpu::<GpuComputeDevice>().expect(gpu_missing);
    let storage = BufferUsage::Storage;
    let params = upload(context, &inputs.params, BufferUsage::Uniform, gpu_missing);
    let positions_in = upload(context, &inputs.positions, storage, gpu_missing);
    let velocities_in = upload(context, &inputs.velocities, storage, gpu_missing);
    let inverse_masses = upload(context, &inputs.inverse_masses, storage, gpu_missing);
    let edge_offsets = upload(context, &inputs.edge_offsets, storage, gpu_missing);
    let edge_neighbor = upload(context, &inputs.edge_neighbor, storage, gpu_missing);
    let edge_params = upload(context, &inputs.edge_params, storage, gpu_missing);
    let positions_out = gpu.allocate_buffer(inputs.positions.len(), storage);
    let velocities_out = gpu.allocate_buffer(inputs.velocities.len(), storage);

    let device = gpu.gpu_driver_device();
    let bind_group = device.create_bind_group(
        &pipeline.bind_group_layout(),
        &[
            &params,
            &positions_in,
            &velocities_in,
            &inverse_masses,
            &edge_offsets,
            &edge_neighbor,
            &edge_params,
            &positions_out,
            &velocities_out,
        ],
    );
    let mut commands = device.create_command_buffer();
    commands.dispatch_compute_with_bind_group(
        pipeline,
        &bind_group,
        (inputs.particle_count as u32)
            .div_ceil(WORKGROUP_SIZE)
            .max(1),
    );
    commands.submit();

    (
        gpu.read_buffer(&positions_out).await,
        gpu.read_buffer(&velocities_out).await,
    )
}
