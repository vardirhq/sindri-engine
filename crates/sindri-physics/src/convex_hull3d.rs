//! Immutable convex geometry for model-derived 3D props.

use rapier3d::prelude as r3;
use thiserror::Error;

use crate::{CollisionLayers, PhysicsMaterial, PhysicsPose3d};

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ConvexHullError3d {
    #[error("convex collision needs at least four points")]
    TooFewPoints,
    #[error("convex collision exceeds 4,096 input points")]
    Limit,
    #[error("convex collision point {0} must be finite and within +/-10,000 units")]
    Point(usize),
    #[error("convex collision needs a volumetric hull with representable mass and inertia")]
    Degenerate,
}

/// A clone-shared convex hull, without an asset or serialized scene reference.
/// Construction fills concavities; it is not convex decomposition.
#[derive(Clone, Debug)]
pub struct ConvexHull3d {
    pub(crate) shape: r3::SharedShape,
}

impl ConvexHull3d {
    /// Computes a hull from at most 4,096 points within +/-10,000 local units.
    /// Duplicates/interior points are allowed. Scale must be baked into points.
    /// Unit-density volume and principal inertia must remain positive and finite,
    /// including their inverses; very thin/tiny or badly conditioned input fails.
    ///
    /// # Errors
    /// Rejects insufficient/oversized input, invalid points, nonvolumetric input
    /// or backend geometry with unusable mass properties.
    pub fn new(points: &[[f32; 3]]) -> Result<Self, ConvexHullError3d> {
        if points.len() < 4 {
            return Err(ConvexHullError3d::TooFewPoints);
        }
        if points.len() > 4_096 {
            return Err(ConvexHullError3d::Limit);
        }
        for (index, point) in points.iter().enumerate() {
            if !point
                .iter()
                .all(|value| value.is_finite() && value.abs() <= 10_000.0)
            {
                return Err(ConvexHullError3d::Point(index));
            }
        }
        let points: Vec<_> = points
            .iter()
            .map(|&point| r3::Vector::from_array(point))
            .collect();
        let shape = r3::SharedShape::convex_hull(&points).ok_or(ConvexHullError3d::Degenerate)?;
        if !usable_mass(shape.mass_properties(1.0)) {
            return Err(ConvexHullError3d::Degenerate);
        }
        Ok(Self { shape })
    }
}

pub(crate) fn usable_mass(value: r3::MassProperties) -> bool {
    let positive = |value: f32| value.is_finite() && value > 0.0;
    positive(value.mass())
        && positive(value.inv_mass)
        && value
            .principal_inertia()
            .to_array()
            .into_iter()
            .all(positive)
        && value
            .inv_principal_inertia
            .to_array()
            .into_iter()
            .all(positive)
        && value.local_com.is_finite()
        && value.principal_inertia_local_frame.is_finite()
}

#[derive(Clone, Debug)]
pub struct ConvexHullCollider3d {
    pub geometry: ConvexHull3d,
    pub pose: PhysicsPose3d,
    pub sensor: bool,
    pub layers: CollisionLayers,
    pub material: PhysicsMaterial,
}

impl ConvexHullCollider3d {
    pub fn new(geometry: ConvexHull3d) -> Self {
        Self {
            geometry,
            pose: PhysicsPose3d::default(),
            sensor: false,
            layers: CollisionLayers::ALL,
            material: PhysicsMaterial {
                friction: 0.5,
                restitution: 0.0,
            },
        }
    }
}
