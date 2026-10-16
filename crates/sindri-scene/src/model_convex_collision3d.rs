//! Model-derived volumetric hulls beside authored bodies and host resources.

use serde::Deserialize;
use sindri_core::{AssetId, EntityId, SceneComponent};
use sindri_physics::{ConvexHull3d, ConvexHullCollider3d, ConvexHullError3d, PhysicsMaterial};
use std::{collections::BTreeMap, sync::Arc};

use crate::{ModelCollider3dComponent, PhysicsSyncError, ScenePhysics3d};

/// One convex approximation of an entire selected model hierarchy. Fills gaps
/// and concavities. An optional rigid body and primitive pieces share its owner;
/// characters, static model collision and voxel collision cannot share it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct ModelConvexCollider3dComponent(pub ModelCollider3dComponent);
impl SceneComponent for ModelConvexCollider3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.model_convex_collider";
}

/// Model-space hull input and immutable unit-scale geometry. Assets, not the
/// scene, expand source node transforms. No triangle or renderer dependency.
#[derive(Debug)]
pub struct ModelConvexGeometry3d {
    points: Vec<[f32; 3]>,
    identity: ConvexHull3d,
}
impl ModelConvexGeometry3d {
    /// # Errors
    /// Has the bounded volumetric contract of `ConvexHull3d::new`.
    pub fn new(points: Vec<[f32; 3]>) -> Result<Self, ConvexHullError3d> {
        let identity = ConvexHull3d::new(&points)?;
        Ok(Self { points, identity })
    }
}

#[derive(Clone, Default)]
pub(crate) struct ModelConvexBindings3d {
    resources: BTreeMap<AssetId, Arc<ModelConvexGeometry3d>>,
    scaled: BTreeMap<(AssetId, [u32; 3]), ConvexHull3d>,
}
impl ModelConvexBindings3d {
    fn bind(&mut self, id: AssetId, resource: Arc<ModelConvexGeometry3d>) {
        self.scaled.retain(|(asset, _), _| asset != &id);
        self.resources.insert(id, resource);
    }
    fn unbind(&mut self, id: &AssetId) {
        self.scaled.retain(|(asset, _), _| asset != id);
        self.resources.remove(id);
    }
    pub(crate) fn prepare(
        &mut self,
        entity: EntityId,
        component: ModelConvexCollider3dComponent,
        scale: [f32; 3],
        previous: Option<&PreparedModelConvex3d>,
    ) -> Result<PreparedModelConvex3d, PhysicsSyncError> {
        let source = self
            .resources
            .get(&component.0.asset)
            .ok_or_else(|| {
                PhysicsSyncError::MissingModelCollision(entity, component.0.asset.clone())
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
            let mut prepared = previous.clone();
            // A registered piece may carry last step's resolved profile. Start
            // each plan from its authored literals before resolving profiles.
            prepared.piece.material = PhysicsMaterial {
                friction: component.0.friction,
                restitution: component.0.restitution,
            };
            return Ok(prepared);
        }
        let key = (component.0.asset.clone(), scale.map(f32::to_bits));
        let geometry = if scale.map(f32::to_bits) == [1.0f32.to_bits(); 3] {
            source.identity.clone()
        } else if let Some(geometry) = self.scaled.get(&key) {
            geometry.clone()
        } else {
            let points: Vec<_> = source
                .points
                .iter()
                .map(|p| std::array::from_fn(|i| p[i] * scale[i]))
                .collect();
            let geometry = ConvexHull3d::new(&points)
                .map_err(|error| PhysicsSyncError::ModelConvexGeometry(entity, error))?;
            if self.scaled.len() >= 64 {
                self.scaled.pop_first();
            }
            self.scaled.insert(key, geometry.clone());
            geometry
        };
        let extent = source
            .points
            .iter()
            .map(|p| {
                p.iter()
                    .zip(scale)
                    .map(|(v, s)| (v * s).powi(2))
                    .sum::<f32>()
                    .sqrt()
            })
            .fold(0.0, f32::max);
        let mut piece = ConvexHullCollider3d::new(geometry);
        piece.sensor = component.0.sensor;
        piece.layers = component.0.layers;
        piece.material = PhysicsMaterial {
            friction: component.0.friction,
            restitution: component.0.restitution,
        };
        Ok(PreparedModelConvex3d {
            source,
            component,
            scale,
            piece,
            extent,
        })
    }
}

#[derive(Clone)]
pub(crate) struct PreparedModelConvex3d {
    source: Arc<ModelConvexGeometry3d>,
    component: ModelConvexCollider3dComponent,
    scale: [f32; 3],
    pub piece: ConvexHullCollider3d,
    pub extent: f32,
}
impl PreparedModelConvex3d {
    pub(crate) fn copy_material_from(&mut self, other: &Self) {
        self.component.0.friction = other.component.0.friction;
        self.component.0.restitution = other.component.0.restitution;
        self.piece.material = other.piece.material;
    }
}
impl PartialEq for PreparedModelConvex3d {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.source, &other.source)
            && self.component == other.component
            && self.piece.material == other.piece.material
            && self.scale.map(f32::to_bits) == other.scale.map(f32::to_bits)
    }
}

impl ScenePhysics3d {
    /// Replaces host geometry; active instances reconcile on a successful step.
    pub fn bind_model_convex_collision(
        &mut self,
        id: AssetId,
        geometry: Arc<ModelConvexGeometry3d>,
    ) {
        self.models.convex.bind(id, geometry);
    }
    /// Active references fail prevalidation while retaining live solver geometry.
    pub fn unbind_model_convex_collision(&mut self, id: &AssetId) {
        self.models.convex.unbind(id);
    }
}
