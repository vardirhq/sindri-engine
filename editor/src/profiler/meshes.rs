use sindri_render::{MeshDrawStats, TexturedCubeRenderer};

use super::Profiler;

/// Scoped 3D work for a view; persistent uploads retain their separate units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshCounters {
    pub meshes: MeshDrawStats,
    pub model_uploads: u64,
    pub cached_geometry_uploads: u64,
}

impl MeshCounters {
    pub(crate) fn snapshot(renderer: &TexturedCubeRenderer) -> Self {
        Self {
            meshes: renderer.draw_stats(),
            model_uploads: renderer.model_cache_stats().uploads,
            cached_geometry_uploads: renderer.cached_mesh_stats().buffer_uploads,
        }
    }

    pub(crate) fn since(self, earlier: Self) -> Self {
        Self {
            meshes: self.meshes.since(earlier.meshes),
            model_uploads: self.model_uploads.saturating_sub(earlier.model_uploads),
            cached_geometry_uploads: self
                .cached_geometry_uploads
                .saturating_sub(earlier.cached_geometry_uploads),
        }
    }

    fn add(&mut self, other: Self) {
        self.meshes.draw_calls += other.meshes.draw_calls;
        self.meshes.render_passes += other.meshes.render_passes;
        self.meshes.uniform_writes += other.meshes.uniform_writes;
        self.meshes.uniform_bytes += other.meshes.uniform_bytes;
        self.meshes.transient_geometry_uploads += other.meshes.transient_geometry_uploads;
        self.model_uploads += other.model_uploads;
        self.cached_geometry_uploads += other.cached_geometry_uploads;
    }
}

impl Profiler {
    /// Adds encoded view work, keeping the Scene view beside Play separate.
    pub(crate) fn add_mesh_counters(&mut self, counters: MeshCounters) {
        let slot = if self.scene_beside_run {
            &mut self.current.scene_mesh_counters
        } else {
            &mut self.current.mesh_counters
        };
        slot.get_or_insert_with(MeshCounters::default).add(counters);
    }
}

impl Profiler {
    /// Records a view's GPU interval separately from CPU work and completion wait.
    pub(crate) fn add_gpu_execution(&mut self, duration: std::time::Duration) {
        let slot = if self.scene_beside_run {
            &mut self.current.scene_gpu_execution
        } else {
            &mut self.current.gpu_execution
        };
        *slot = Some(slot.unwrap_or_default() + duration);
    }
}

impl Profiler {
    pub(crate) fn add_batch_counters(&mut self, counters: sindri_render::BatchDrawStats) {
        let slot = if self.scene_beside_run {
            &mut self.current.scene_batch_counters
        } else {
            &mut self.current.batch_counters
        };
        *slot = Some(slot.unwrap_or_default().combined(counters));
    }
}
