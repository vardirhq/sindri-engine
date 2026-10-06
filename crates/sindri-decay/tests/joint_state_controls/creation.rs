//! Distance creation validates before authoring and retains solver ownership rules.
use super::*;

const CREATE: &str = r#"Physics.create_distance_joint(this.entity, World.find("Anchor"), World.find("Body"), 10.0);"#;

pub(super) fn empty_fixture() -> (SceneExtractor, World, EntityId, EntityId) {
    let (extractor, mut world, owner, body) = fixture(KINDS[0]);
    world.get_mut(owner).unwrap().components.remove(KINDS[0]);
    world.get_mut(body).unwrap().name = Some("Body".into());
    let anchor = world
        .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
        .unwrap();
    world.get_mut(anchor).unwrap().name = Some("Anchor".into());
    (extractor, world, owner, body)
}

pub(super) fn run_call(
    world: &mut World,
    extractor: &SceneExtractor,
    physics: &mut ScenePhysics2d,
    call: &str,
    succeeds: bool,
) {
    let mut sources = ScriptSources::new();
    sources.insert(
        "joint.decay",
        format!("script Joint {{ fn start() {{ {call} }} }}"),
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

#[test]
fn creation_and_recreation_keep_bodies_motion_and_legacy_constraints() {
    for built in [false, true] {
        let (extractor, mut world, owner, body) = empty_fixture();
        let anchor = world
            .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
            .unwrap();
        let other_components = world.get(owner).unwrap().components.clone();
        let mut physics = ScenePhysics2d::top_down().unwrap();
        physics
            .world_mut()
            .remember_distance_joint(anchor, body, 10.0)
            .unwrap();
        if built {
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            physics
                .world_mut()
                .set_linear_velocity(body, [3.0, 0.0])
                .unwrap();
        }
        for _ in 0..2 {
            let before_count = physics.world().joint_count();
            run_call(&mut world, &extractor, &mut physics, CREATE, true);
            let mut expected = other_components.clone();
            expected.insert(
                KINDS[0].into(),
                json!({"first":"anchor", "second":"body", "enabled":true, "max_distance":10.0}),
            );
            assert_eq!(world.get(owner).unwrap().components, expected);
            assert_eq!(physics.world().joint_count(), before_count);
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 2);
            if built {
                assert!(physics.world().linear_velocity(body).unwrap()[0] > 2.9);
            }
            run_call(
                &mut world,
                &extractor,
                &mut physics,
                "Physics.remove_joint(this.entity);",
                true,
            );
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 1);
        }
    }
}

#[test]
fn existing_kinds_and_invalid_arguments_fail_before_any_mutation() {
    for kind in KINDS {
        let (extractor, mut world, owner, _) = fixture(kind);
        let before = world.get(owner).unwrap().components.clone();
        run_call(
            &mut world,
            &extractor,
            &mut ScenePhysics2d::top_down().unwrap(),
            CREATE,
            false,
        );
        assert_eq!(world.get(owner).unwrap().components, before);
    }
    for args in [
        r#"this.entity, World.find("Anchor"), World.find("Body"), 0.0"#,
        r#"this.entity, World.find("Anchor"), World.find("Body"), -1.0"#,
        r#"this.entity, World.find("Anchor"), World.find("Body"), 1e40"#,
        r#"this.entity, World.find("Anchor"), World.find("Anchor"), 1.0"#,
        r#"null, World.find("Anchor"), World.find("Body"), 1.0"#,
    ] {
        let (extractor, mut world, owner, _) = empty_fixture();
        let before = world.get(owner).unwrap().components.clone();
        let call = format!("Physics.create_distance_joint({args});");
        run_call(
            &mut world,
            &extractor,
            &mut ScenePhysics2d::top_down().unwrap(),
            &call,
            false,
        );
        assert_eq!(world.get(owner).unwrap().components, before);
    }
}

#[test]
fn unbound_inactive_and_missing_physics_have_explicit_contracts() {
    let (extractor, mut world, owner, body) = empty_fixture();
    let before = world.get(owner).unwrap().components.clone();
    let mut sources = ScriptSources::new();
    sources.insert(
        "joint.decay",
        format!("script Joint {{ fn start() {{ {CREATE} }} }}"),
    );
    let report = Scripts::new().advance(
        &mut world,
        extractor.components(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(!report.failures.is_empty());
    assert_eq!(world.get(owner).unwrap().components, before);
    let mut physics = ScenePhysics2d::top_down().unwrap();
    world.get_mut(body).unwrap().disabled = true;
    run_call(&mut world, &extractor, &mut physics, CREATE, true);
    physics
        .step(&mut world, extractor.components(), STEP)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    world.get_mut(body).unwrap().disabled = false;
    physics
        .step(&mut world, extractor.components(), STEP)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    run_call(
        &mut world,
        &extractor,
        &mut physics,
        "Physics.remove_joint(this.entity); Physics.create_distance_joint(this.entity, null, null, 1.0);",
        true,
    );
    physics
        .step(&mut world, extractor.components(), STEP)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    assert_eq!(world.get(owner).unwrap().components[KINDS[0]]["first"], "");
}

#[test]
fn unstable_and_out_of_scope_references_are_rejected_atomically() {
    for outside in [false, true] {
        let (extractor, mut world, owner, body) = empty_fixture();
        if outside {
            let root = world.spawn(EntityData::default());
            world.set_parent(body, Some(root)).unwrap();
        } else {
            world.get_mut(body).unwrap().source_id = None;
        }
        let before = world.get(owner).unwrap().components.clone();
        run_call(
            &mut world,
            &extractor,
            &mut ScenePhysics2d::top_down().unwrap(),
            CREATE,
            false,
        );
        assert_eq!(world.get(owner).unwrap().components, before);
    }
}

#[test]
fn runtime_prefab_creation_keeps_local_paths_and_rejects_other_instances() {
    for (kind, call) in [
        (KINDS[0], CREATE),
        (KINDS[1], super::hinge_creation::CREATE),
        (KINDS[3], super::spring_creation::CREATE),
        (KINDS[2], super::slider_creation::CREATE),
    ] {
        let (extractor, mut template, owner, body) = empty_fixture();
        let root = template
            .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
            .unwrap();
        template.set_parent(owner, Some(root)).unwrap();
        template.set_parent(body, Some(root)).unwrap();
        let prefab = sindri_core::PrefabDocument {
            entities: template.to_scene().unwrap().entities,
            ..sindri_core::PrefabDocument::default()
        };
        let mut world = World::default();
        let first = world.spawn_prefab(&prefab).unwrap();
        let second = world.spawn_prefab(&prefab).unwrap();
        let key = SceneEntityId::new("joint").unwrap();
        let first_owner = first.by_source_id[&key];
        let second_owner = second.by_source_id[&key];
        world
            .get_mut(second_owner)
            .unwrap()
            .components
            .remove(ScriptComponent::TYPE_NAME);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        run_call(&mut world, &extractor, &mut physics, call, true);
        assert_eq!(
            world.get(first_owner).unwrap().components[kind]["second"],
            "body"
        );
        physics
            .step(&mut world, extractor.components(), STEP)
            .unwrap();
        assert_eq!(physics.world().joint_count(), 1);
        let script = world
            .get_mut(first_owner)
            .unwrap()
            .components
            .remove(ScriptComponent::TYPE_NAME)
            .unwrap();
        world
            .get_mut(second_owner)
            .unwrap()
            .components
            .insert(ScriptComponent::TYPE_NAME.into(), script);
        let before = world.get(second_owner).unwrap().components.clone();
        run_call(&mut world, &extractor, &mut physics, call, false);
        assert_eq!(world.get(second_owner).unwrap().components, before);
        assert_eq!(physics.world().joint_count(), 1);
    }
}
