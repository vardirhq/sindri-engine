//! Physical work recorded by sprite, shape and glyph batch encoders.

/// Lifetime counters, excluding resource initialization and atlas/texture uploads.
/// Empty batches record nothing. Submission slot resets preserve these counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BatchDrawStats {
    pub draw_calls: u64,
    pub render_passes: u64,
    pub instances: u64,
    pub buffer_writes: u64,
    pub uniform_bytes: u64,
    pub instance_bytes: u64,
}

impl BatchDrawStats {
    /// Work recorded since a snapshot from the same renderer or renderer group.
    #[must_use]
    pub const fn since(self, earlier: Self) -> Self {
        Self {
            draw_calls: self.draw_calls.saturating_sub(earlier.draw_calls),
            render_passes: self.render_passes.saturating_sub(earlier.render_passes),
            instances: self.instances.saturating_sub(earlier.instances),
            buffer_writes: self.buffer_writes.saturating_sub(earlier.buffer_writes),
            uniform_bytes: self.uniform_bytes.saturating_sub(earlier.uniform_bytes),
            instance_bytes: self.instance_bytes.saturating_sub(earlier.instance_bytes),
        }
    }

    /// Combines independent renderer snapshots or view deltas.
    #[must_use]
    pub const fn combined(self, other: Self) -> Self {
        Self {
            draw_calls: self.draw_calls.saturating_add(other.draw_calls),
            render_passes: self.render_passes.saturating_add(other.render_passes),
            instances: self.instances.saturating_add(other.instances),
            buffer_writes: self.buffer_writes.saturating_add(other.buffer_writes),
            uniform_bytes: self.uniform_bytes.saturating_add(other.uniform_bytes),
            instance_bytes: self.instance_bytes.saturating_add(other.instance_bytes),
        }
    }

    pub(crate) fn record_draw(
        &mut self,
        instances: u32,
        uniform_bytes: usize,
        instance_bytes: usize,
    ) {
        self.draw_calls += 1;
        self.render_passes += 1;
        self.instances += u64::from(instances);
        self.buffer_writes += 2;
        self.uniform_bytes += u64::try_from(uniform_bytes).unwrap_or(u64::MAX);
        self.instance_bytes += u64::try_from(instance_bytes).unwrap_or(u64::MAX);
    }
}
