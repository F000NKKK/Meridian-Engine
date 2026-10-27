//! Tests of this module.

use meridian_platform_core::BackendCapabilities;

use super::*;

/// Every test here needs a real adapter; some CI/sandboxed
/// environments have none. Skip rather than fail in that case.
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
    assert!(!device.adapter_name().is_empty());
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
async fn texture_2d_upload_and_sampler_creation_do_not_panic() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    // 2x2 RGBA8: enough to exercise write_texture's row-layout math
    // without needing a real decoded image asset.
    let pixels: [u8; 16] = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let texture = device.create_texture_2d(2, 2);
    device.write_texture(&texture, &pixels);
    let _sampler = device.create_sampler();
}
