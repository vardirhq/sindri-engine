use super::support::{Fixture, STEP, near, position};
use crate::Character2dComponent;
use serde_json::json;
use sindri_core::SceneComponent;

#[test]
fn invalid_controller_configuration_is_rejected_before_registration() {
    for kind in ["body", "missing", "sensor", "compound"] {
        let mut f = Fixture::new();
        let hero = f.character([0.0, 1.0]);
        match kind {
            "body" => f.body(hero, "dynamic", [0.0; 2]),
            "missing" => {
                f.world
                    .get_mut(hero)
                    .unwrap()
                    .components
                    .remove("sindri.physics2d.collider");
            }
            "sensor" => {
                f.world
                    .get_mut(hero)
                    .unwrap()
                    .components
                    .get_mut("sindri.physics2d.collider")
                    .unwrap()["sensor"] = json!(true);
            }
            _ => {
                let piece =
                    f.world.get(hero).unwrap().components["sindri.physics2d.collider"].clone();
                *f.world
                    .get_mut(hero)
                    .unwrap()
                    .components
                    .get_mut("sindri.physics2d.collider")
                    .unwrap() = json!({"pieces": [piece, piece]});
            }
        }
        assert!(f.physics.step(&mut f.world, &f.components, STEP).is_err());
        assert!(f.physics.world().pose(hero).is_err());
    }
    for payload in [
        json!({"skin": 0.0}),
        json!({"max_iterations": 0}),
        json!({"up": [0.0, 2.0]}),
        json!({"step_height": -1.0}),
    ] {
        assert!(serde_json::from_value::<Character2dComponent>(payload).is_err());
    }
}

#[test]
fn teleport_and_collider_rebuild_clear_old_support() {
    for rebuild in [false, true] {
        let mut f = Fixture::new();
        let hero = f.character([0.0, 0.61]);
        let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
        f.body(platform, "kinematic_velocity", [1.0, 0.0]);
        f.step();
        if rebuild {
            f.world
                .get_mut(hero)
                .unwrap()
                .components
                .get_mut("sindri.physics2d.collider")
                .unwrap()["friction"] = json!(0.7);
            f.world
                .get_mut(hero)
                .unwrap()
                .components
                .get_mut("sindri.physics2d.collider")
                .unwrap()["rotation"] = json!(0.1);
        } else {
            f.set_position(hero, [0.0, 2.0]);
        }
        f.step();
        let expected = if rebuild {
            2.0 * STEP.as_secs_f32()
        } else {
            0.0
        };
        near(position(&f.world, hero)[0], expected);
        if !rebuild {
            assert!(f.physics.character_motion(hero).unwrap().platform.is_none());
        }
    }
}

#[test]
fn support_rebuild_clears_rider_snapshot_without_replaying_old_motion() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [1.0, 0.0]);
    f.step();
    f.world
        .get_mut(platform)
        .unwrap()
        .components
        .get_mut("sindri.physics2d.collider")
        .unwrap()["shape"]["half_extents"] = json!([3.0, 0.1]);
    f.step();
    near(
        f.physics
            .character_motion(hero)
            .unwrap()
            .platform
            .as_ref()
            .unwrap()
            .motion
            .translation[0],
        STEP.as_secs_f32(),
    );
    near(position(&f.world, hero)[0], 2.0 * STEP.as_secs_f32());
}

#[test]
fn disable_remove_and_reenable_discard_queued_input_and_runtime_state() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    f.step();
    f.queue(hero, [5.0, 0.0], false);
    f.world.get_mut(hero).unwrap().disabled = true;
    f.step();
    assert!(f.physics.character_motion(hero).is_none());
    assert!(f.physics.world().pose(hero).is_err());
    f.world.get_mut(hero).unwrap().disabled = false;
    f.step();
    near(position(&f.world, hero)[0], 0.0);
    f.world
        .get_mut(hero)
        .unwrap()
        .components
        .remove(Character2dComponent::TYPE_NAME);
    f.queue(hero, [5.0, 0.0], false);
    f.step();
    assert!(f.physics.character_motion(hero).is_none());
    near(position(&f.world, hero)[0], 0.0);
}

#[test]
fn reparenting_while_preserving_world_pose_clears_carry_snapshot() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    let parent = f.empty([10.0, 0.0]);
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [1.0, 0.0]);
    f.step();
    f.world
        .set_parent_keeping_place(hero, Some(parent))
        .unwrap();
    f.step();
    near(position(&f.world, hero)[0], 2.0 * STEP.as_secs_f32());
    near(
        f.physics
            .character_motion(hero)
            .unwrap()
            .platform
            .as_ref()
            .unwrap()
            .motion
            .translation[0],
        STEP.as_secs_f32(),
    );
}

#[test]
fn despawned_handles_cannot_pass_requests_or_drop_timers_to_a_reused_slot() {
    let mut f = Fixture::new();
    let old = f.character([0.0, 0.61]);
    f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.step();
    f.physics
        .character_requests()
        .drop_through(old, 1.0)
        .unwrap();
    f.queue(old, [5.0, -1.0], false);
    f.world.despawn_recursive(old).unwrap();
    let new = f.character([0.0, 0.61]);
    assert_ne!(old, new);
    f.step();
    assert!(f.physics.character_motion(old).is_none());
    assert!(f.physics.character_motion(new).unwrap().grounded);
    near(position(&f.world, new)[0], 0.0);
}

#[test]
fn direct_backend_actor_teleports_also_forget_old_support() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [1.0, 0.0]);
    f.step();
    f.physics
        .world_mut()
        .move_to(
            hero,
            sindri_physics::PhysicsPose2d {
                position: [0.0, 2.0],
                rotation: 0.0,
            },
        )
        .unwrap();
    f.step();
    assert!(f.physics.character_motion(hero).unwrap().platform.is_none());
    near(position(&f.world, hero)[0], 0.0);
}

#[test]
fn supporting_parent_motion_is_applied_once_and_keeps_local_coordinates() {
    let mut f = Fixture::new();
    let platform = f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.body(platform, "kinematic_velocity", [1.0, 0.0]);
    let hero = f.character([0.0, 0.61]);
    f.world.set_parent(hero, Some(platform)).unwrap();
    f.step();
    f.step();
    near(position(&f.world, hero)[0], 2.0 * STEP.as_secs_f32());
    near(
        f.world.get(hero).unwrap().transform_3d.unwrap().position[0],
        0.0,
    );
}

#[test]
fn script_borrows_share_cached_results_and_queue_only_the_next_step() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 0.61]);
    f.box_collider([0.0, 0.0], [2.0, 0.1]);
    f.step();
    let (physics, _, requests, motions) = f.physics.for_scripts_with_characters();
    assert!(motions.get(hero).unwrap().grounded);
    requests.move_character(hero, [0.2, 0.0], false).unwrap();
    near(physics.pose(hero).unwrap().position[0], 0.0);
    near(motions.get(hero).unwrap().translation[0], 0.0);
    f.step();
    near(position(&f.world, hero)[0], 0.2);
}

#[test]
fn saving_preserves_unknown_authored_fields_without_runtime_requests_or_handles() {
    let mut f = Fixture::new();
    let hero = f.character([0.0, 1.0]);
    let authored = json!({"skin": 0.02, "snap_distance": 0.3, "extension": {"value": 42}});
    f.world
        .get_mut(hero)
        .unwrap()
        .components
        .insert(Character2dComponent::TYPE_NAME.into(), authored.clone());
    f.world.assign_missing_source_ids("saved").unwrap();
    f.queue(hero, [0.2, 0.0], false);
    f.physics
        .character_requests()
        .drop_through(hero, 1.0)
        .unwrap();
    f.step();
    let document = f.world.to_scene().unwrap();
    let id = f.world.get(hero).unwrap().source_id.as_ref().unwrap();
    assert_eq!(
        document.entity(id).unwrap().components[Character2dComponent::TYPE_NAME],
        authored
    );
    let text = document.to_canonical_json().unwrap();
    let reopened = sindri_core::SceneDocument::from_json(&text).unwrap();
    let mut world = sindri_core::World::default();
    sindri_core::LoadedScenes::new()
        .enter_with(&mut world, "reopened", &reopened, &sindri_core::NoPrefabs)
        .unwrap();
    let mut physics = crate::ScenePhysics2d::top_down().unwrap();
    physics.step(&mut world, &f.components, STEP).unwrap();
    let hero = world
        .entities()
        .find(|(_, data)| {
            data.components
                .contains_key(Character2dComponent::TYPE_NAME)
        })
        .unwrap()
        .0;
    near(physics.character_motion(hero).unwrap().translation[0], 0.0);
    near(position(&world, hero)[0], 0.2);
}
