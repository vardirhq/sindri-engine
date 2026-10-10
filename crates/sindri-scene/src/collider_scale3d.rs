//! Resolve authored local collider geometry into solver units.

use glam::{Mat3, Quat, Vec3};
use sindri_physics::{Collider3d, ColliderShape3d};
use thiserror::Error;

use crate::Collider3dComponent;

const TOLERANCE: f32 = 1.0e-5;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum ColliderScaleError3d {
    #[error("transform scale must be finite and positive on every axis")]
    Scale,
    #[error("piece {index}: {reason}")]
    Piece { index: usize, reason: &'static str },
}

impl Collider3dComponent {
    /// Applies the composed entity scale to local dimensions and offsets.
    /// Boxes support nonuniform scale when their rotated axes remain orthogonal.
    /// Spheres and capsules require uniform scale; ellipsoids, sheared boxes and
    /// stretched capsule caps cannot be represented by the current shape model.
    /// Invalid/overflowing geometry fails without changing authored pieces.
    pub fn scaled(&self, scale: [f32; 3]) -> Result<Vec<Collider3d>, ColliderScaleError3d> {
        if !scale.into_iter().all(|axis| axis.is_finite() && axis > 0.0) {
            return Err(ColliderScaleError3d::Scale);
        }
        self.0
            .iter()
            .enumerate()
            .map(|(index, piece)| {
                scaled_piece(*piece, Vec3::from_array(scale))
                    .map_err(|reason| ColliderScaleError3d::Piece { index, reason })
            })
            .collect()
    }
}

fn scaled_piece(mut piece: Collider3d, scale: Vec3) -> Result<Collider3d, &'static str> {
    let rotation = Quat::from_array(piece.rotation);
    let norm = piece
        .rotation
        .into_iter()
        .map(|part| f64::from(part).powi(2))
        .sum::<f64>();
    if !rotation.is_finite() || (norm - 1.0).abs() > 0.0001 {
        return Err("rotation must be a finite unit quaternion");
    }
    piece.offset = (Vec3::from_array(piece.offset) * scale).to_array();
    if !piece.offset.into_iter().all(f32::is_finite) {
        return Err("scaled offset is not finite");
    }
    piece.shape = match piece.shape {
        ColliderShape3d::Box { half_extents } => {
            let rotation = Mat3::from_quat(rotation.normalize());
            let axes = [rotation.x_axis, rotation.y_axis, rotation.z_axis].map(|axis| axis * scale);
            // Normalize before dot products: huge finite scales must not
            // overflow a squared length or hide a sheared basis.
            let lengths = axes.map(|axis| axis.to_array().into_iter().fold(0.0, f32::hypot));
            if !lengths
                .into_iter()
                .all(|length| length.is_finite() && length > 0.0)
            {
                return Err("scaled box axes are not finite and positive");
            }
            let unit: [Vec3; 3] = std::array::from_fn(|i| axes[i] / lengths[i]);
            if [(0, 1), (0, 2), (1, 2)]
                .into_iter()
                .any(|(a, b)| unit[a].dot(unit[b]).abs() > TOLERANCE)
            {
                return Err("nonuniform scale shears the rotated box");
            }
            piece.rotation = Quat::from_mat3(&Mat3::from_cols(unit[0], unit[1], unit[2]))
                .normalize()
                .to_array();
            ColliderShape3d::Box {
                half_extents: std::array::from_fn(|i| half_extents[i] * lengths[i]),
            }
        }
        ColliderShape3d::Sphere { radius } => ColliderShape3d::Sphere {
            radius: radius * uniform(scale)?,
        },
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => {
            let factor = uniform(scale)?;
            ColliderShape3d::Capsule {
                half_height: half_height * factor,
                radius: radius * factor,
            }
        }
    };
    let dimensions = match piece.shape {
        ColliderShape3d::Box { half_extents } => half_extents,
        ColliderShape3d::Sphere { radius } => [radius; 3],
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => [half_height, radius, radius],
    };
    if !dimensions
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
    {
        return Err("scaled dimensions must be finite and positive");
    }
    Ok(piece)
}

fn uniform(scale: Vec3) -> Result<f32, &'static str> {
    if (scale.max_element() - scale.min_element()) / scale.max_element() > TOLERANCE {
        return Err("spheres and capsules require uniform scale");
    }
    Ok(scale.x)
}
