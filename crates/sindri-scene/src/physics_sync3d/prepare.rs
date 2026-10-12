//! Prevalidate scene body ownership before solver reconciliation.

use super::{Authored, body_at, pose_of};
use crate::{Character3dComponent, Collider3dComponent, PhysicsSyncError, RigidBody3dComponent};
use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::{PhysicsWorld3d, RigidBody3d, RigidBodyKind};
use std::collections::BTreeMap;

pub(super) fn prepare(
    world: &World,
    components: &ComponentSchemaRegistry,
    physics: &PhysicsWorld3d,
    models: &mut crate::ModelCollisionBindings3d,
    registered: &BTreeMap<EntityId, Authored>,
) -> Result<BTreeMap<EntityId, Authored>, PhysicsSyncError> {
    let mut plan = BTreeMap::new();
    for (entity, collider) in components.query::<Collider3dComponent>(world)? {
        let character = components.get::<Character3dComponent>(world, entity)?;
        let mut body = components
            .get::<RigidBody3dComponent>(world, entity)?
            .map(|value| value.0);
        if character.is_some() && body.is_none() {
            body = Some(RigidBody3d {
                kind: RigidBodyKind::KinematicVelocity,
                ..RigidBody3d::default()
            });
        }
        let pose = pose_of(world, entity, body);
        let placed = body_at(body, pose);
        if placed.kind != RigidBodyKind::Static
            && world
                .get(entity)
                .and_then(|data| data.transform_3d)
                .is_some_and(|transform| transform.z_locked)
        {
            return Err(PhysicsSyncError::LockedDepth3d(entity));
        }
        physics.validate_insertion(entity, placed, &collider.0)?;
        let scale = world.world_transform(entity).unwrap_or_default().scale;
        let pieces = collider
            .scaled(scale)
            .map_err(|error| PhysicsSyncError::ColliderScale3d(entity, error))?;
        physics.validate_insertion(entity, placed, &pieces)?;
        plan.insert(
            entity,
            Authored {
                body,
                pieces,
                model: None,
                convex: None,
            },
        );
    }
    for (entity, component) in components.query::<crate::ModelCollider3dComponent>(world)? {
        let model = prepare_model(
            world,
            physics,
            models,
            entity,
            component,
            registered
                .get(&entity)
                .and_then(|authored| authored.model.as_ref()),
        )?;
        plan.insert(
            entity,
            Authored {
                body: None,
                pieces: Vec::new(),
                model: Some(model),
                convex: None,
            },
        );
    }
    for (entity, component) in components.query::<crate::ModelConvexCollider3dComponent>(world)? {
        let authored = prepare_convex(
            world,
            components,
            physics,
            models,
            entity,
            component,
            (plan.remove(&entity), registered.get(&entity)),
        )?;
        plan.insert(entity, authored);
    }
    Ok(plan)
}

fn prepare_model(
    world: &World,
    physics: &PhysicsWorld3d,
    models: &mut crate::ModelCollisionBindings3d,
    entity: EntityId,
    component: crate::ModelCollider3dComponent,
    previous: Option<&crate::model_collision3d::PreparedModelCollider3d>,
) -> Result<crate::model_collision3d::PreparedModelCollider3d, PhysicsSyncError> {
    let data = world
        .get(entity)
        .ok_or(sindri_physics::PhysicsError::MissingEntity(entity))?;
    if [
        <crate::ModelConvexCollider3dComponent as sindri_core::SceneComponent>::TYPE_NAME,
        <Collider3dComponent as sindri_core::SceneComponent>::TYPE_NAME,
        <RigidBody3dComponent as sindri_core::SceneComponent>::TYPE_NAME,
        <Character3dComponent as sindri_core::SceneComponent>::TYPE_NAME,
        <crate::VoxelCollider3dComponent as sindri_core::SceneComponent>::TYPE_NAME,
    ]
    .iter()
    .any(|name| data.components.contains_key(*name))
    {
        return Err(PhysicsSyncError::InvalidModelCollider(
            entity,
            "static model collision owns its body; remove competing 3D physics components",
        ));
    }
    let scale = world.world_transform(entity).unwrap_or_default().scale;
    let model = models.prepare(entity, component, scale, previous)?;
    physics.validate_static_mesh_insertion(
        entity,
        pose_of(world, entity, None),
        std::slice::from_ref(&model.piece),
    )?;
    Ok(model)
}

fn prepare_convex(
    world: &World,
    components: &ComponentSchemaRegistry,
    physics: &PhysicsWorld3d,
    models: &mut crate::ModelCollisionBindings3d,
    entity: EntityId,
    component: crate::ModelConvexCollider3dComponent,
    existing: (Option<Authored>, Option<&Authored>),
) -> Result<Authored, PhysicsSyncError> {
    use sindri_core::SceneComponent;
    let (primitive, previous) = existing;
    let data = world
        .get(entity)
        .ok_or(sindri_physics::PhysicsError::MissingEntity(entity))?;
    if [
        crate::ModelCollider3dComponent::TYPE_NAME,
        Character3dComponent::TYPE_NAME,
        crate::VoxelCollider3dComponent::TYPE_NAME,
    ]
    .iter()
    .any(|name| data.components.contains_key(*name))
    {
        return Err(PhysicsSyncError::InvalidModelCollider(
            entity,
            "convex model collision cannot share a character, static model or voxel owner",
        ));
    }
    let body = components
        .get::<RigidBody3dComponent>(world, entity)?
        .map(|value| value.0);
    let placed = body_at(body, pose_of(world, entity, body));
    if placed.kind != RigidBodyKind::Static
        && data
            .transform_3d
            .is_some_and(|transform| transform.z_locked)
    {
        return Err(PhysicsSyncError::LockedDepth3d(entity));
    }
    let scale = world.world_transform(entity).unwrap_or_default().scale;
    let convex = models.convex.prepare(
        entity,
        component,
        scale,
        previous.and_then(|value| value.convex.as_ref()),
    )?;
    let pieces = primitive.map_or_else(Vec::new, |value| value.pieces);
    physics.validate_hull_body(entity, placed, &pieces, std::slice::from_ref(&convex.piece))?;
    Ok(Authored {
        body,
        pieces,
        model: None,
        convex: Some(convex),
    })
}
