//! Authored voxel worlds as collision geometry in the scene solver.

use super::tests::{components, spawn};
use super::*;
use crate::{
    TileSetBindings, VoxelCollider3dComponent, VoxelCollisionError3d, VoxelGround,
    VoxelWorldComponent,
};
use serde_json::{Value, json};
use sindri_core::{EntityData, SceneComponent, TileSetDocument, Transform3D};
use sindri_physics::{PhysicsEventKind, RaycastFilter3d};

const STEP: Duration = Duration::from_millis(10);

fn generator(surface: &Value) -> Value {
    json!({
        "kind": "layered_terrain",
        "seed": 7,
        "base_height": 4,
        "height_variation": 0,
        "surface_voxel": surface,
        "subsurface_voxel": surface,
        "deep_voxel": surface,
        "subsurface_depth": 2,
    })
}

fn voxel_world(world: &mut World, voxels: Value, collider: bool) -> EntityId {
    let mut data = EntityData {
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    };
    data.components
        .insert(VoxelWorldComponent::TYPE_NAME.into(), voxels);
    if collider {
        data.components.insert(
            VoxelCollider3dComponent::TYPE_NAME.into(),
            components()
                .default_payload(VoxelCollider3dComponent::TYPE_NAME)
                .unwrap()
                .clone(),
        );
    }
    world.spawn(data)
}

fn materials(edits: &Value) -> Value {
    json!({"generator": generator(&json!(1)), "edits": edits})
}

/// The level of the flat surface block, and the height of its top.
fn surface(voxels: &Value, tile_sets: Option<&TileSetBindings>) -> (i32, f32) {
    let component: VoxelWorldComponent = serde_json::from_value(voxels.clone()).unwrap();
    let (level, top, _) = VoxelGround::of(&component, tile_sets)
        .unwrap()
        .surface(0, 0)
        .unwrap();
    (level, top)
}

fn ball(world: &mut World, position: [f32; 3]) -> EntityId {
    spawn(
        world,
        position,
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.25),
    )
}

fn run(
    physics: &mut ScenePhysics3d,
    world: &mut World,
    tile_sets: Option<&TileSetBindings>,
    steps: usize,
) -> Vec<PhysicsEvent3d> {
    let registry = components();
    let mut events = Vec::new();
    for _ in 0..steps {
        physics
            .step_with_tile_sets(world, &registry, tile_sets, STEP)
            .unwrap();
        events.extend_from_slice(physics.events());
    }
    events
}

fn height(world: &World, entity: EntityId) -> f32 {
    world.world_transform(entity).unwrap().position[1]
}

fn earth() -> ScenePhysics3d {
    ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap()
}

#[test]
fn a_body_lands_on_authored_voxels_then_falls_down_a_dug_shaft() {
    let mut world = World::default();
    let voxels = materials(&json!([]));
    let (level, top) = surface(&voxels, None);
    let floor = voxel_world(&mut world, voxels, true);
    let body = ball(&mut world, [0.5, top + 2.0, 0.5]);
    let mut physics = earth();

    // Long enough for the body to fall asleep on the ground.
    let events = run(&mut physics, &mut world, None, 400);
    assert!(
        (height(&world, body) - (top + 0.25)).abs() < 0.05,
        "rests on the surface"
    );
    assert!(events.iter().any(|event| {
        event.kind == PhysicsEventKind::CollisionStarted
            && [event.first, event.second].contains(&floor)
            && [event.first, event.second].contains(&body)
    }));
    let hit = physics
        .world()
        .raycast(
            [0.5, top + 5.0, 3.5],
            [0.0, -1.0, 0.0],
            20.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
        .expect("resident voxels answer queries");
    assert_eq!(hit.entity, floor);

    // A shaft one voxel wide and six deep, edited into the authored world.
    let shaft: Vec<_> = (level - 5..=level)
        .map(|y| json!({"at": [0, y, 0], "block": ""}))
        .collect();
    let payload = world.get_mut(floor).unwrap();
    payload
        .components
        .get_mut(VoxelWorldComponent::TYPE_NAME)
        .unwrap()["edits"] = json!(shaft);
    run(&mut physics, &mut world, None, 150);
    assert!(
        (height(&world, body) - (top - 6.0 + 0.25)).abs() < 0.05,
        "lands on the shaft's floor: {}",
        height(&world, body)
    );
}

#[test]
fn residency_follows_bodies_far_from_the_origin() {
    let mut world = World::default();
    let voxels = materials(&json!([]));
    let (_, top) = surface(&voxels, None);
    voxel_world(&mut world, voxels, true);
    let near = ball(&mut world, [0.5, top + 1.0, 0.5]);
    let far = ball(&mut world, [4_000.5, top + 1.0, -2_500.5]);
    let mut physics = earth();
    run(&mut physics, &mut world, None, 120);
    for body in [near, far] {
        assert!((height(&world, body) - (top + 0.25)).abs() < 0.05);
    }
}

#[test]
fn a_block_set_decides_what_collides_and_its_shape() {
    let mut tile_sets = TileSetBindings::new();
    let face = r#"{ "sprite": "b.png#top", "size": [1.0, 1.0] }"#;
    let document = TileSetDocument::from_json(&format!(
        r#"{{ "format_version": 1, "tiles": {{
             "stone": {{ "faces": {{ "top": {face} }} }},
             "water": {{ "faces": {{ "top": {face} }}, "walkable": false, "collides": false }},
             "slab": {{ "faces": {{ "top": {face} }}, "height": 0.5 }}
           }} }}"#
    ))
    .unwrap();
    tile_sets.bind("b.tileset", document).unwrap();
    let mut world = World::default();
    let bare = json!({"blocks": "b.tileset", "generator": generator(&json!("stone"))});
    let (level, top) = surface(&bare, Some(&tile_sets));
    let mut voxels = bare;
    // Water replaces one surface block, and a slab stands on another.
    voxels["edits"] = json!([
        {"at": [0, level, 0], "block": "water"},
        {"at": [3, level + 1, 0], "block": "slab"},
    ]);
    voxel_world(&mut world, voxels, true);
    let sinking = ball(&mut world, [0.5, top + 1.0, 0.5]);
    let resting = ball(&mut world, [3.5, top + 1.5, 0.5]);
    let mut physics = earth();
    run(&mut physics, &mut world, Some(&tile_sets), 150);
    assert!((height(&world, sinking) - (top - 1.0 + 0.25)).abs() < 0.05);
    assert!((height(&world, resting) - (top + 0.5 + 0.25)).abs() < 0.05);
}

#[test]
fn removing_the_voxel_collider_releases_its_geometry() {
    let mut world = World::default();
    let voxels = materials(&json!([]));
    let (_, top) = surface(&voxels, None);
    let floor = voxel_world(&mut world, voxels, true);
    let body = ball(&mut world, [0.5, top + 0.5, 0.5]);
    let mut physics = earth();
    run(&mut physics, &mut world, None, 80);
    assert!((height(&world, body) - (top + 0.25)).abs() < 0.05);
    assert!(physics.world().contains(floor));
    world
        .get_mut(floor)
        .unwrap()
        .components
        .remove(VoxelCollider3dComponent::TYPE_NAME);
    run(&mut physics, &mut world, None, 50);
    assert!(!physics.world().contains(floor));
    assert!(
        height(&world, body) < top - 0.5,
        "{} vs {top}",
        height(&world, body)
    );
}

#[test]
fn invalid_voxel_owners_fail_before_any_body_changes() {
    let registry = components();
    for case in 0..3 {
        let mut world = World::default();
        let floor = match case {
            0 => {
                // A collider with no voxel world to collide with.
                let floor = voxel_world(&mut world, json!(null), true);
                world
                    .get_mut(floor)
                    .unwrap()
                    .components
                    .remove(VoxelWorldComponent::TYPE_NAME);
                floor
            }
            1 => {
                let floor = voxel_world(&mut world, materials(&json!([])), true);
                world.get_mut(floor).unwrap().components.insert(
                    Collider3dComponent::TYPE_NAME.into(),
                    json!({"pieces": [Collider3d::sphere(1.0)]}),
                );
                floor
            }
            _ => {
                // A block set nobody bound.
                voxel_world(
                    &mut world,
                    json!({"blocks": "missing.tileset", "generator": generator(&json!("stone"))}),
                    true,
                )
            }
        };
        let body = ball(&mut world, [0.5, 30.0, 0.5]);
        let mut physics = earth();
        let error = physics
            .step_with_tile_sets(&mut world, &registry, None, STEP)
            .unwrap_err();
        assert!(
            matches!(
                (case, &error),
                (0, PhysicsSyncError::MissingVoxelWorld(owner))
                    | (1, PhysicsSyncError::ConflictingVoxelOwner(owner))
                    | (2, PhysicsSyncError::VoxelWorld(owner, _))
                    if *owner == floor
            ),
            "{case}: {error}"
        );
        assert!(!physics.world().contains(body), "{case}");
        assert!(physics.world().is_empty(), "{case}");
    }
}

#[test]
fn a_window_larger_than_the_budget_is_refused_whole() {
    let registry = components();
    let mut world = World::default();
    voxel_world(&mut world, materials(&json!([])), true);
    // A body this large would need more sections than the default budget.
    let giant = spawn(
        &mut world,
        [0.0, 10.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(80.0),
    );
    let mut physics = earth();
    let error = physics
        .step_with_tile_sets(&mut world, &registry, None, STEP)
        .unwrap_err();
    assert!(matches!(
        error,
        PhysicsSyncError::VoxelCollision(VoxelCollisionError3d::Budget("sections"))
    ));
    assert!(!physics.world().contains(giant));
}
