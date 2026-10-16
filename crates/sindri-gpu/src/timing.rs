//! Optional native command-encoder timestamps for synchronous benchmarks.

use std::{sync::mpsc, time::Duration};

use thiserror::Error;

/// Both features must be enabled on the device, not only advertised by the adapter.
pub const TIMING_FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);

/// Reusable timestamp pair. Begin, finish, submit and read before reusing it.
///
/// This measures the bracketed GPU command interval, excluding query resolution,
/// readback and presentation. Backend command reordering and timestamp precision
/// can affect the interval; it is not a sum of individual shader execution times.
#[derive(Debug)]
pub struct GpuTimer {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    period: f32,
}

impl GpuTimer {
    /// Returns `None` when the requested device lacks encoder timestamp support.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(TIMING_FEATURES) {
            return None;
        }
        Some(Self {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Sindri benchmark timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolved: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Sindri benchmark timestamp resolve"),
                size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Sindri benchmark timestamp readback"),
                size: 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            period: queue.get_timestamp_period(),
        })
    }

    pub fn begin(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.queries, 0);
    }

    /// Ends the interval and records its resolution/copy outside the interval.
    pub fn finish(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.queries, 1);
        encoder.resolve_query_set(&self.queries, 0..2, &self.resolved, 0);
        encoder.copy_buffer_to_buffer(&self.resolved, 0, &self.readback, 0, 16);
    }

    /// Blocks for readback. Intended for benchmarks, not an interactive frame loop.
    pub fn read(&self, device: &wgpu::Device) -> Result<Duration, GpuTimingError> {
        let result = self.read_mapped(device);
        self.readback.unmap();
        result
    }

    fn read_mapped(&self, device: &wgpu::Device) -> Result<Duration, GpuTimingError> {
        let slice = self.readback.slice(..);
        let (sender, receiver) = mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| GpuTimingError::Readback(error.to_string()))?;
        receiver
            .recv()
            .map_err(|error| GpuTimingError::Readback(error.to_string()))?
            .map_err(|error| GpuTimingError::Readback(error.to_string()))?;
        slice
            .get_mapped_range()
            .map_err(|error| GpuTimingError::Readback(error.to_string()))
            .and_then(|bytes| elapsed(&bytes, self.period))
    }
}

#[allow(clippy::cast_precision_loss)] // A duration's u64 tick count converts to f64 for a fractional period.
fn elapsed(bytes: &[u8], period: f32) -> Result<Duration, GpuTimingError> {
    let ticks = |offset| {
        bytes
            .get(offset..offset + 8)
            .and_then(|value| value.try_into().ok())
            .map(u64::from_ne_bytes)
            .ok_or(GpuTimingError::InvalidTimestamps)
    };
    if !period.is_finite() || period <= 0.0 {
        return Err(GpuTimingError::InvalidTimestamps);
    }
    let delta = ticks(8)?.wrapping_sub(ticks(0)?);
    Duration::try_from_secs_f64(delta as f64 * f64::from(period) / 1_000_000_000.0)
        .map_err(|_| GpuTimingError::InvalidTimestamps)
}

#[derive(Debug, Error)]
pub enum GpuTimingError {
    #[error("GPU timestamp readback failed: {0}")]
    Readback(String),
    #[error("invalid GPU timestamp data or period")]
    InvalidTimestamps,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_use_the_queue_period_and_allow_counter_wrap() {
        let bytes = [100_u64.to_ne_bytes(), 140_u64.to_ne_bytes()].concat();
        assert_eq!(elapsed(&bytes, 2.5).unwrap(), Duration::from_nanos(100));
        let wrapped = [(u64::MAX - 3).to_ne_bytes(), 6_u64.to_ne_bytes()].concat();
        assert_eq!(elapsed(&wrapped, 1.0).unwrap(), Duration::from_nanos(10));
        assert!(elapsed(&bytes, f32::NAN).is_err());
        assert!(elapsed(&bytes, 0.0).is_err());
        assert!(elapsed(&[], 1.0).is_err());
    }
}
