//! Dimension-independent, asset-free collision coefficients.

use serde::{Deserialize, Serialize};

use crate::PhysicsError;
use crate::validate::{non_negative, normalized};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PhysicsMaterial {
    pub friction: f32,
    pub restitution: f32,
}

impl PhysicsMaterial {
    /// Friction is finite and non-negative; restitution is finite in `[0, 1]`.
    ///
    /// # Errors
    /// Returns the offending coefficient's validation error.
    pub fn validate(self) -> Result<(), PhysicsError> {
        non_negative("friction", self.friction)?;
        normalized("restitution", self.restitution)
    }
}

#[cfg(test)]
mod tests {
    use sindri_core::EntityId;

    use super::*;
    use crate::{Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d};

    #[test]
    fn material_edits_keep_attached_distance_joints() {
        let anchor = EntityId::from_bits(1 << 32);
        let body = EntityId::from_bits(2 << 32);
        let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
        world
            .insert_static_collider(anchor, PhysicsPose2d::default(), &[Collider2d::circle(0.1)])
            .unwrap();
        world
            .insert_body(
                body,
                RigidBody2d {
                    pose: PhysicsPose2d {
                        position: [1.0, 0.0],
                        rotation: 0.0,
                    },
                    ..RigidBody2d::default()
                },
                &[Collider2d::circle(0.1)],
            )
            .unwrap();
        world.connect_distance(anchor, body, 2.0).unwrap();
        world
            .set_materials(
                body,
                &[PhysicsMaterial {
                    friction: 0.0,
                    restitution: 0.8,
                }],
            )
            .unwrap();
        world.set_linear_velocity(body, [10.0, 0.0]).unwrap();
        for _ in 0..60 {
            world.step(std::time::Duration::from_millis(16)).unwrap();
        }
        let [x, y] = world.pose(body).unwrap().position;
        assert!(
            x.hypot(y) < 2.1,
            "distance joint was lost after coefficient edit"
        );
    }

    #[test]
    fn invalid_updates_leave_every_piece_unchanged() {
        let entity = EntityId::from_bits(1 << 32);
        let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
        world
            .insert_body(
                entity,
                RigidBody2d::default(),
                &[Collider2d::circle(1.0), Collider2d::circle(0.5)],
            )
            .unwrap();
        let original = world.materials(entity).unwrap();
        let good = PhysicsMaterial {
            friction: 0.9,
            restitution: 0.8,
        };
        for invalid in [
            PhysicsMaterial {
                friction: f32::NAN,
                ..good
            },
            PhysicsMaterial {
                restitution: -0.1,
                ..good
            },
            PhysicsMaterial {
                friction: f32::INFINITY,
                ..good
            },
        ] {
            assert!(world.set_materials(entity, &[good, invalid]).is_err());
            assert_eq!(world.materials(entity).unwrap(), original);
        }
        assert!(world.set_materials(entity, &[good]).is_err());
        assert_eq!(world.materials(entity).unwrap(), original);
        assert!(
            world
                .set_materials(EntityId::from_bits(2 << 32), &[good])
                .is_err()
        );
    }
}
