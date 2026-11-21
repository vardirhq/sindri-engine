use super::*;
use crate::{VoxelBlock, VoxelEdit};

fn built(blocks: &[[i32; 3]]) -> VoxelWorldComponent {
    let mut world: VoxelWorldComponent =
        serde_json::from_value(serde_json::json!({ "generator": { "kind": "built" } })).unwrap();
    world.edits = blocks
        .iter()
        .map(|&at| VoxelEdit {
            at,
            block: VoxelBlock::Material(1),
        })
        .collect();
    world
}

fn volume(pieces: &[Collider3d]) -> f32 {
    pieces
        .iter()
        .map(|piece| match piece.shape {
            sindri_physics::ColliderShape3d::Box {
                half_extents: [x, y, z],
            } => 8.0 * x * y * z,
            _ => panic!("voxel bodies are made of boxes"),
        })
        .sum()
}

#[test]
fn a_solid_block_of_blocks_is_one_box() {
    let mut blocks = Vec::new();
    for x in 0..4 {
        for y in 0..2 {
            for z in 0..3 {
                blocks.push([x, y, z]);
            }
        }
    }
    let pieces = voxel_body_pieces(&built(&blocks), &VoxelBody3dComponent::default()).unwrap();
    assert_eq!(pieces.len(), 1);
    let offset = pieces[0].offset;
    assert!(
        (0..3).all(|axis| (offset[axis] - [2.0, 1.0, 1.5][axis]).abs() < 1e-6),
        "{offset:?}"
    );
    assert!((volume(&pieces) - 24.0).abs() < 1e-5);
}

#[test]
fn every_block_is_covered_once_whatever_the_shape() {
    // An L of blocks with a hole, and a block removed by a later edit.
    let mut world = built(&[
        [0, 0, 0],
        [1, 0, 0],
        [2, 0, 0],
        [0, 0, 1],
        [0, 1, 0],
        [5, 5, 5],
    ]);
    world.edits.push(VoxelEdit {
        at: [5, 5, 5],
        block: VoxelBlock::air(),
    });
    let pieces = voxel_body_pieces(&world, &VoxelBody3dComponent::default()).unwrap();
    assert!((volume(&pieces) - 5.0).abs() < 1e-5, "{pieces:?}");
    assert!(pieces.len() <= 3, "{pieces:?}");
    // The same blocks always make the same boxes.
    assert_eq!(
        pieces,
        voxel_body_pieces(&world, &VoxelBody3dComponent::default()).unwrap()
    );
}

#[test]
fn only_a_built_world_is_a_body() {
    let mut world = built(&[[0, 0, 0]]);
    world.generator = VoxelGeneratorDocument::default();
    assert!(voxel_body_pieces(&world, &VoxelBody3dComponent::default()).is_err());
    let empty = built(&[]);
    assert!(
        voxel_body_pieces(&empty, &VoxelBody3dComponent::default())
            .unwrap()
            .is_empty()
    );
}

mod in_a_scene {
    use std::time::Duration;

    use serde_json::json;
    use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};

    use crate::{
        Collider3dComponent, RigidBody3dComponent, SceneExtractor, ScenePhysics3d,
        VoxelBody3dComponent, VoxelWorldComponent,
    };

    const STEP: Duration = Duration::from_millis(16);

    /// A ship of half-metre blocks: a 4 x 1 x 2 deck.
    fn ship(world: &mut World, kind: &str, velocity: [f32; 3]) -> EntityId {
        let edits: Vec<_> = (0..4)
            .flat_map(|x| (0..2).map(move |z| json!({ "at": [x, 0, z], "block": 1 })))
            .collect();
        world.spawn(EntityData {
            transform_3d: Some(Transform3D {
                position: [0.0, 5.0, 0.0],
                scale: [0.5; 3],
                ..Transform3D::default()
            }),
            components: [
                (
                    VoxelWorldComponent::TYPE_NAME.into(),
                    json!({ "generator": { "kind": "built" }, "edits": edits }),
                ),
                (VoxelBody3dComponent::TYPE_NAME.into(), json!({})),
                (
                    RigidBody3dComponent::TYPE_NAME.into(),
                    json!({ "kind": kind, "position": [0, 0, 0], "rotation": [0, 0, 0, 1],
                            "linear_velocity": velocity, "angular_velocity": [0, 0, 0],
                            "gravity_scale": 1.0, "linear_damping": 0.0,
                            "angular_damping": 0.0, "lock_rotation": false }),
                ),
            ]
            .into(),
            ..EntityData::default()
        })
    }

    fn floor(world: &mut World) {
        world.spawn(EntityData {
            transform_3d: Some(Transform3D::default()),
            components: [(
                Collider3dComponent::TYPE_NAME.into(),
                json!({ "pieces": [sindri_physics::Collider3d::cuboid([20.0, 0.5, 20.0])] }),
            )]
            .into(),
            ..EntityData::default()
        });
    }

    fn run(physics: &mut ScenePhysics3d, world: &mut World, steps: usize) {
        let registry = SceneExtractor::new().unwrap().components().clone();
        for _ in 0..steps {
            physics.step(world, &registry, STEP).unwrap();
        }
    }

    #[test]
    fn a_built_structure_falls_and_lands_as_one_body() {
        let mut world = World::default();
        floor(&mut world);
        let ship = ship(&mut world, "dynamic", [0.0; 3]);
        let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
        run(&mut physics, &mut world, 240);
        let resting = world.world_transform(ship).unwrap().position;
        // The deck's half-metre blocks are 0..0.5 high above the entity: it
        // rests with its underside on the floor's top at 0.5.
        assert!((resting[1] - 0.5).abs() < 0.05, "{resting:?}");
        // Eight half-metre blocks (an eighth of a cubic metre each) of
        // density one.
        let mass = physics.world().mass(ship).unwrap();
        assert!((mass - 1.0).abs() < 1e-3, "{mass}");
    }

    #[test]
    fn placing_a_block_in_flight_keeps_the_ship_moving() {
        let mut world = World::default();
        let ship = ship(&mut world, "dynamic", [2.0, 0.0, 0.0]);
        let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
        run(&mut physics, &mut world, 30);
        let before = physics.world().linear_velocity(ship).unwrap();
        world
            .get_mut(ship)
            .unwrap()
            .components
            .get_mut(VoxelWorldComponent::TYPE_NAME)
            .unwrap()["edits"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "at": [0, 1, 0], "block": 1 }));
        run(&mut physics, &mut world, 1);
        let after = physics.world().linear_velocity(ship).unwrap();
        assert!((after[0] - before[0]).abs() < 0.05, "{before:?} {after:?}");
        assert!(
            (physics.world().mass(ship).unwrap() - 1.125).abs() < 1e-3,
            "the block adds its mass"
        );
    }

    #[test]
    fn a_voxel_body_refuses_competing_colliders_and_generated_worlds() {
        let mut world = World::default();
        let ship = ship(&mut world, "dynamic", [0.0; 3]);
        world.get_mut(ship).unwrap().components.insert(
            Collider3dComponent::TYPE_NAME.into(),
            json!({ "pieces": [sindri_physics::Collider3d::sphere(1.0)] }),
        );
        let registry = SceneExtractor::new().unwrap().components().clone();
        let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
        assert!(physics.step(&mut world, &registry, STEP).is_err());
    }
}
