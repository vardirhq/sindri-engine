//! Component removal keeps owner/body state and legacy constraint ownership.
use super::*;

#[test]
fn every_kind_removes_before_and_after_sync_without_removing_legacy_joints() {
    for kind in KINDS {
        for built in [false, true] {
            let (extractor, mut world, owner, body) = fixture(kind);
            let anchor = world
                .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
                .unwrap();
            let mut physics = ScenePhysics2d::top_down().unwrap();
            physics
                .world_mut()
                .remember_distance_joint(anchor, body, 10.0)
                .unwrap();
            if built {
                physics
                    .step(&mut world, extractor.components(), STEP)
                    .unwrap();
                assert_eq!(physics.world().joint_count(), 2);
                physics
                    .world_mut()
                    .set_linear_velocity(body, [3.0, 0.0])
                    .unwrap();
            }
            let data = world.get(owner).unwrap().clone();
            let mut expected = data.components.clone();
            expected.remove(kind);
            remove(&mut world, &extractor, &mut physics, true);
            assert_eq!(world.get(owner).unwrap().components, expected);
            assert_eq!(world.get(owner).unwrap().source_id, data.source_id);
            assert!(world.contains(anchor) && world.contains(body));
            if built {
                assert_eq!(
                    physics.world().joint_count(),
                    2,
                    "released only at fixed synchronization"
                );
                assert!(physics.world().linear_velocity(body).unwrap()[0] > 2.9);
            }
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(
                physics.world().joint_count(),
                1,
                "legacy constraint remains"
            );
            if built {
                assert!(physics.world().linear_velocity(body).unwrap()[0] > 2.9);
            }
            remove(&mut world, &extractor, &mut physics, false);
            assert_eq!(world.get(owner).unwrap().components, expected);
        }
    }
}

#[test]
fn suspended_removal_and_missing_physics_have_explicit_contracts() {
    for kind in KINDS {
        let (extractor, mut world, owner, _) = fixture(kind);
        world
            .get_mut(owner)
            .unwrap()
            .components
            .get_mut(kind)
            .unwrap()["enabled"] = json!(false);
        let original = world.get(owner).unwrap().components.clone();
        let mut sources = ScriptSources::new();
        sources.insert(
            "joint.decay",
            "script Joint { fn start() { Physics.remove_joint(this.entity); } }",
        );
        let report = Scripts::new().advance(
            &mut world,
            extractor.components(),
            ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
        );
        assert!(!report.failures.is_empty());
        assert_eq!(world.get(owner).unwrap().components, original);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        remove(&mut world, &extractor, &mut physics, true);
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
        assert_eq!(physics.world().joint_count(), 0);
        assert!(!world.get(owner).unwrap().components.contains_key(kind));
    }
}

fn remove(
    world: &mut World,
    extractor: &SceneExtractor,
    physics: &mut ScenePhysics2d,
    succeeds: bool,
) {
    let mut sources = ScriptSources::new();
    sources.insert(
        "joint.decay",
        "script Joint { fn start() { Physics.remove_joint(this.entity); } }",
    );
    advance(
        &mut Scripts::new(),
        world,
        extractor,
        &sources,
        &InputState::default(),
        physics,
        succeeds,
    );
}
