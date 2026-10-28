//! Tests of this module.

use meridian_platform_winit::BackendCapabilities;

use super::*;

/// Every test here needs a real adapter; some CI/sandboxed
/// environments have none (no GPU, no software rasterizer ICD
/// installed). Skip rather than fail in that case — this validates
/// this crate's own plumbing, not that a GPU is present.
#[cfg(feature = "hw-tests")]
async fn device_or_skip() -> Option<Device> {
    match Device::new().await {
        Ok(device) => Some(device),
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            None
        }
    }
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn device_reports_a_nonempty_adapter_name() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    assert_ne!(device.adapter_name(), "");
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn device_reports_gpu_capabilities_matching_adapter_name() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let gpu = device
        .gpu()
        .expect("a constructed Device always has a real adapter");
    assert_eq!(gpu.device_name, device.adapter_name());
    assert!(device.gpu_available());
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn buffer_write_then_read_round_trips() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let buffer = device.create_buffer(data.len(), BufferUsage::Storage);
    device.write_buffer(&buffer, &data);
    let read_back = device.read_buffer(&buffer).await;
    assert_eq!(read_back, data);
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn texture_reports_requested_dimensions() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let texture = device.create_texture(
        64,
        32,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
    );
    assert_eq!(texture.width, 64);
    assert_eq!(texture.height, 32);
}

/// The actual end-to-end proof this module exists to give: write
/// data to a buffer, run a real compute shader over it on the GPU,
/// read the result back, and check it matches what the shader says
/// it does — not just "didn't panic".
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn compute_dispatch_doubles_every_element() {
    const SHADER: &str = r"
        @group(0) @binding(0)
        var<storage, read_write> data: array<u32>;

        @compute @workgroup_size(4)
        fn double_elements(@builtin(global_invocation_id) id: vec3<u32>) {
            data[id.x] = data[id.x] * 2u;
        }
    ";

    let Some(device) = device_or_skip().await else {
        return;
    };

    let input: [u32; 4] = [1, 2, 3, 4];
    let bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

    let buffer = device.create_buffer(bytes.len(), BufferUsage::Storage);
    device.write_buffer(&buffer, &bytes);

    let shader = device.create_shader("double_elements", SHADER);
    let pipeline = device.create_compute_pipeline(&shader, "double_elements");

    let mut commands = device.create_command_buffer();
    commands.dispatch_compute(&pipeline, &buffer, 1);
    commands.submit();

    let result_bytes = device.read_buffer(&buffer).await;
    let result: Vec<u32> = result_bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect();
    assert_eq!(result, vec![2, 4, 6, 8]);
}
