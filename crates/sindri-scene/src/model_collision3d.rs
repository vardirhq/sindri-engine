//! Immutable CPU model geometry and scene-owned static collision bindings.

use serde::Deserialize;
use sindri_core::{AssetId, EntityId, SceneComponent};
use sindri_physics::{
    CollisionLayers, PhysicsMaterial, TriangleMesh3d, TriangleMeshCollider3d, TriangleMeshError3d,
};
use std::{collections::BTreeMap, sync::Arc};

use crate::{PhysicsSyncError, ScenePhysics3d};

/// Static collision from a logical model asset. No rigid body or inline collider
/// may share the owner; a separate render model reference is optional.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ModelCollider3dComponent {
    pub asset: AssetId,
    #[serde(default = "half")]
    pub friction: f32,
    #[serde(default)]
    pub restitution: f32,
    #[serde(default = "layers")]
    pub layers: CollisionLayers,
    #[serde(default)]
    pub sensor: bool,
}
const fn half() -> f32 {
    0.5
}
const fn layers() -> CollisionLayers {
    CollisionLayers::ALL
}
impl SceneComponent for ModelCollider3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.model_collider";
}

/// Model-space CPU triangles. Assets prepare node transforms before this seam.
/// The scene retains input for positive nonuniform scale; backend types stay private.
#[derive(Debug)]
pub struct ModelCollisionGeometry3d {
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    identity: TriangleMesh3d,
}
impl ModelCollisionGeometry3d {
    /// # Errors
    /// Has the bounded geometry contract of `TriangleMesh3d::new`.
    pub fn new(
        positions: Vec<[f32; 3]>,
        triangles: Vec<[u32; 3]>,
    ) -> Result<Self, TriangleMeshError3d> {
        let identity = TriangleMesh3d::new(&positions, &triangles)?;
        Ok(Self {
            positions,
            triangles,
            identity,
        })
    }
}

/// Host-delivered logical resources and a bounded cache of scaled mesh variants.
#[derive(Clone, Default)]
pub struct ModelCollisionBindings3d {
    resources: BTreeMap<AssetId, Arc<ModelCollisionGeometry3d>>,
    scaled: BTreeMap<(AssetId, [u32; 3]), TriangleMesh3d>,
    pub(crate) convex: crate::model_convex_collision3d::ModelConvexBindings3d,
}
impl ModelCollisionBindings3d {
    /// Replaces a logical resource and drops its cached scaled variants.
    /// Active geometry reconciles on the next successful scene step.
    pub fn bind(
        &mut self,
        asset: AssetId,
        resource: Arc<ModelCollisionGeometry3d>,
    ) -> Option<Arc<ModelCollisionGeometry3d>> {
        self.scaled.retain(|(id, _), _| id != &asset);
        self.resources.insert(asset, resource)
    }
    /// Removes a binding; active references will fail scene prevalidation.
    pub fn unbind(&mut self, asset: &AssetId) -> Option<Arc<ModelCollisionGeometry3d>> {
        self.scaled.retain(|(id, _), _| id != asset);
        self.resources.remove(asset)
    }
    pub(crate) fn prepare(
        &mut self,
        entity: EntityId,
        component: ModelCollider3dComponent,
        scale: [f32; 3],
        previous: Option<&PreparedModelCollider3d>,
    ) -> Result<PreparedModelCollider3d, PhysicsSyncError> {
        let source = self
            .resources
            .get(&component.asset)
            .ok_or_else(|| {
                PhysicsSyncError::MissingModelCollision(entity, component.asset.clone())
            })?
            .clone();
        if !scale.iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(PhysicsSyncError::InvalidModelCollider(
                entity,
                "scale must be finite and positive",
            ));
        }
        if let Some(previous) = previous
            && Arc::ptr_eq(&source, &previous.source)
            && previous.component == component
            && previous.scale.map(f32::to_bits) == scale.map(f32::to_bits)
        {
            return Ok(previous.clone());
        }
        let key = (component.asset.clone(), scale.map(f32::to_bits));
        let geometry = if scale.map(f32::to_bits) == [1.0f32.to_bits(); 3] {
            source.identity.clone()
        } else if let Some(geometry) = self.scaled.get(&key) {
            geometry.clone()
        } else {
            let positions: Vec<_> = source
                .positions
                .iter()
                .map(|p| std::array::from_fn(|i| p[i] * scale[i]))
                .collect();
            let geometry = TriangleMesh3d::new(&positions, &source.triangles)
                .map_err(|error| PhysicsSyncError::ModelCollisionGeometry(entity, error))?;
            // Bound idle variants. Active colliders retain their shared resources.
            if self.scaled.len() >= 64 {
                self.scaled.pop_first();
            }
            self.scaled.insert(key, geometry.clone());
            geometry
        };
        let mut piece = TriangleMeshCollider3d::new(geometry);
        piece.sensor = component.sensor;
        piece.layers = component.layers;
        piece.material = PhysicsMaterial {
            friction: component.friction,
            restitution: component.restitution,
        };
        Ok(PreparedModelCollider3d {
            source,
            component,
            scale,
            piece,
        })
    }
}

#[derive(Clone)]
pub(crate) struct PreparedModelCollider3d {
    source: Arc<ModelCollisionGeometry3d>,
    component: ModelCollider3dComponent,
    scale: [f32; 3],
    pub piece: TriangleMeshCollider3d,
}
impl PartialEq for PreparedModelCollider3d {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.source, &other.source)
            && self.component == other.component
            && self.scale.map(f32::to_bits) == other.scale.map(f32::to_bits)
    }
}

impl ScenePhysics3d {
    /// Host-delivered resources, shared by scene instances and checkpoints.
    #[must_use]
    pub const fn model_collision_bindings(&self) -> &ModelCollisionBindings3d {
        &self.models
    }
    /// Delivers resources independently of authored entity and solver state.
    pub fn model_collision_bindings_mut(&mut self) -> &mut ModelCollisionBindings3d {
        &mut self.models
    }
}
