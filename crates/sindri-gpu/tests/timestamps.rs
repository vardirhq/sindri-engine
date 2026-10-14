//! Real timestamp resolution, readback and repeated-buffer reuse.

use sindri_gpu::{GpuContext, GpuRequestOptions, GpuTimer, TIMING_FEATURES};
use sindri_render::{ClearOperations, DepthTarget, OffscreenTarget, encode_clear};

fn gpu(optional_features: wgpu::Features) -> Option<GpuContext> {
    match pollster::block_on(GpuContext::request(
        &wgpu::Instance::default(),
        None,
        &GpuRequestOptions {
            optional_features,
            ..GpuRequestOptions::default()
        },
    )) {
        Ok(gpu) => Some(gpu),
        Err(error) => {
            assert!(
                std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
                "required GPU unavailable: {error}"
            );
            None
        }
    }
}

#[test]
fn a_baseline_device_keeps_timing_unavailable() {
    let Some(gpu) = gpu(wgpu::Features::empty()) else {
        return;
    };
    assert!(GpuTimer::new(&gpu.device, &gpu.queue).is_none());
}

#[test]
fn optional_timestamps_resolve_across_consecutive_submissions() {
    let Some(gpu) = gpu(TIMING_FEATURES) else {
        return;
    };
    let Some(timer) = GpuTimer::new(&gpu.device, &gpu.queue) else {
        assert!(
            std::env::var_os("SINDRI_REQUIRE_GPU_TIMESTAMPS").is_none(),
            "required timestamps unavailable"
        );
        return;
    };
    let target = OffscreenTarget::new(&gpu.device, 128, 128).unwrap();
    let depth = DepthTarget::new(&gpu.device, 128, 128);
    for _ in 0..3 {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        timer.begin(&mut encoder);
        encode_clear(
            &mut encoder,
            target.view(),
            &depth,
            ClearOperations::default(),
        );
        timer.finish(&mut encoder);
        gpu.queue.submit([encoder.finish()]);
        let duration = timer.read(&gpu.device).unwrap();
        assert!(
            !duration.is_zero(),
            "GPU commands produce an actual interval"
        );
    }
}
