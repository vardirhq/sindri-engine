//! Physical work recorded by the textured 3D and imported-model encoders.

/// Cumulative counters; subtract snapshots around encoding a frame.
///
/// These exclude frame clears, sprites, glyphs, shapes, post-processing and
/// resource initialization. Uniform bytes count queue-write payloads, without
/// driver padding. Geometry uploads count transient vertex/index buffer pairs;
/// persistent model, retained inline and voxel uploads have their own cache counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshDrawStats {
    pub draw_calls: u64,
    /// Includes the directional shadow clear, which has no draw call.
    pub render_passes: u64,
    pub uniform_writes: u64,
    pub uniform_bytes: u64,
    pub transient_geometry_uploads: u64,
}

impl MeshDrawStats {
    /// Work since an earlier snapshot from the same renderer.
    #[must_use]
    pub const fn since(self, earlier: Self) -> Self {
        Self {
            draw_calls: self.draw_calls.saturating_sub(earlier.draw_calls),
            render_passes: self.render_passes.saturating_sub(earlier.render_passes),
            uniform_writes: self.uniform_writes.saturating_sub(earlier.uniform_writes),
            uniform_bytes: self.uniform_bytes.saturating_sub(earlier.uniform_bytes),
            transient_geometry_uploads: self
                .transient_geometry_uploads
                .saturating_sub(earlier.transient_geometry_uploads),
        }
    }

    pub(crate) fn record_draw(&mut self, uniform_bytes: usize) {
        self.draw_calls += 1;
        self.render_passes += 1;
        self.uniform_writes += 1;
        self.uniform_bytes += u64::try_from(uniform_bytes).unwrap_or(u64::MAX);
    }

    pub(crate) const fn and(self, other: Self) -> Self {
        Self {
            draw_calls: self.draw_calls.saturating_add(other.draw_calls),
            render_passes: self.render_passes.saturating_add(other.render_passes),
            uniform_writes: self.uniform_writes.saturating_add(other.uniform_writes),
            uniform_bytes: self.uniform_bytes.saturating_add(other.uniform_bytes),
            transient_geometry_uploads: self
                .transient_geometry_uploads
                .saturating_add(other.transient_geometry_uploads),
        }
    }
}
