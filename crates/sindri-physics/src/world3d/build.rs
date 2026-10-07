//! The only conversions from validated Sindri 3D values into backend builders.

use super::r3;
use crate::{Collider3d, ColliderShape3d, PhysicsPose3d, RigidBody3d, RigidBodyKind};
use sindri_core::EntityId;

pub(super) fn pose(value: PhysicsPose3d) -> r3::Pose {
    r3::Pose::from_parts(
        r3::Vector::from_array(value.position),
        r3::Rotation::from_array(value.rotation).normalize(),
    )
}

pub(super) fn body(value: RigidBody3d) -> r3::RigidBodyBuilder {
    let builder = match value.kind {
        RigidBodyKind::Static => r3::RigidBodyBuilder::fixed(),
        RigidBodyKind::Dynamic => r3::RigidBodyBuilder::dynamic(),
        RigidBodyKind::KinematicPosition => r3::RigidBodyBuilder::kinematic_position_based(),
        RigidBodyKind::KinematicVelocity => r3::RigidBodyBuilder::kinematic_velocity_based(),
    }
    .pose(pose(PhysicsPose3d {
        position: value.position,
        rotation: value.rotation,
    }))
    .linvel(r3::Vector::from_array(value.linear_velocity))
    .angvel(r3::Vector::from_array(value.angular_velocity))
    .gravity_scale(value.gravity_scale)
    .linear_damping(value.linear_damping)
    .angular_damping(value.angular_damping);
    if value.lock_rotation {
        builder.angvel(r3::Vector::ZERO).lock_rotations()
    } else {
        builder
    }
}

pub(super) fn collider(entity: EntityId, value: Collider3d) -> r3::ColliderBuilder {
    let builder = match value.shape {
        ColliderShape3d::Box {
            half_extents: [x, y, z],
        } => r3::ColliderBuilder::cuboid(x, y, z),
        ColliderShape3d::Sphere { radius } => r3::ColliderBuilder::ball(radius),
        ColliderShape3d::Capsule {
            half_height,
            radius,
        } => r3::ColliderBuilder::capsule_y(half_height, radius),
    };
    builder
        .position(pose(PhysicsPose3d {
            position: value.offset,
            rotation: value.rotation,
        }))
        .sensor(value.sensor)
        .collision_groups(r3::InteractionGroups::new(
            r3::Group::from_bits_retain(value.layers.memberships),
            r3::Group::from_bits_retain(value.layers.filter),
            r3::InteractionTestMode::And,
        ))
        .active_collision_types(r3::ActiveCollisionTypes::all())
        .active_events(r3::ActiveEvents::COLLISION_EVENTS)
        .friction(value.friction)
        .friction_combine_rule(r3::CoefficientCombineRule::Min)
        .restitution(value.restitution)
        .user_data(u128::from(entity.to_bits()))
}
