//! 2D shadow maps kept from one frame to the next.
//!
//! A lamp post and the trees around it do not move, so the shadow map built
//! for its light is the same every frame, and building them was most of what
//! extracting a lit 2D frame cost. A map is kept while its light stands where
//! it stood, reaches as far, and has exactly the same occluder edges within its
//! reach; anything else builds it again. Only the maps the latest frame used
//! are kept, so the cache never outgrows one frame's lights.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// What a shadow map is built from, as bits, so equal inputs are one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ShadowKey {
    position: [u32; 2],
    radius: u32,
    source_radius: u32,
    /// A fingerprint of the edges within the light's reach.
    edges: u64,
}

impl ShadowKey {
    fn new(position: [f32; 2], radius: f32, source_radius: f32, edges: &[[[f32; 2]; 2]]) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        edges.len().hash(&mut hasher);
        for point in edges.iter().flatten().flatten() {
            point.to_bits().hash(&mut hasher);
        }
        Self {
            position: position.map(f32::to_bits),
            radius: radius.to_bits(),
            source_radius: source_radius.to_bits(),
            edges: hasher.finish(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct ShadowMapCache {
    /// The maps the latest finished frame used.
    kept: HashMap<ShadowKey, Arc<[f32]>>,
    /// The maps the frame being extracted has used so far.
    used: HashMap<ShadowKey, Arc<[f32]>>,
}

impl ShadowMapCache {
    /// The map for a light at `position` reaching `radius` past `edges`,
    /// from the last frame when nothing it depends on changed, else from
    /// `build`.
    pub(super) fn map(
        &mut self,
        position: [f32; 2],
        radius: f32,
        source_radius: f32,
        edges: &[[[f32; 2]; 2]],
        build: impl FnOnce() -> Vec<f32>,
    ) -> Arc<[f32]> {
        let key = ShadowKey::new(position, radius, source_radius, edges);
        let map = self
            .used
            .get(&key)
            .or_else(|| self.kept.get(&key))
            .cloned()
            .unwrap_or_else(|| Arc::from(build()));
        self.used.insert(key, Arc::clone(&map));
        map
    }

    /// Ends a frame: what it used is what the next may reuse.
    pub(super) fn finish_frame(&mut self) {
        self.kept = std::mem::take(&mut self.used);
    }
}

#[cfg(test)]
mod tests {
    use super::ShadowMapCache;

    #[test]
    fn a_map_is_reused_until_its_light_or_its_edges_change() {
        let mut cache = ShadowMapCache::default();
        let edges = [[[1.0, 0.0], [1.0, 1.0]]];
        let mut built = 0;
        let mut frame = |cache: &mut ShadowMapCache, at: [f32; 2], edges: &[[[f32; 2]; 2]]| {
            cache.map(at, 4.0, 0.3, edges, || {
                built += 1;
                vec![0.0]
            });
            cache.finish_frame();
        };
        frame(&mut cache, [0.0, 0.0], &edges);
        frame(&mut cache, [0.0, 0.0], &edges);
        frame(&mut cache, [0.5, 0.0], &edges);
        frame(&mut cache, [0.5, 0.0], &[[[2.0, 0.0], [2.0, 1.0]]]);
        assert_eq!(
            built, 3,
            "built once, reused, then rebuilt for a move and an edge"
        );
    }
}
