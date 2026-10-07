//! Endpoint retargeting preserves authored state and rejects invalid scope atomically.
use super::*;

#[test]
fn retarget_and_clear_work_for_every_kind_before_and_after_sync() {
    for kind in KINDS {
        for built in [false, true] {
            let (extractor, mut world, owner, moving) = fixture(kind);
            let anchor = world
                .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
                .unwrap();
            world.get_mut(anchor).unwrap().name = Some("Anchor".into());
            world.get_mut(moving).unwrap().name = Some("Body".into());
            let mut data = world.get(anchor).unwrap().clone();
            data.source_id = Some(SceneEntityId::new("replacement").unwrap());
            data.name = Some("Replacement".into());
            data.disabled = true;
            let replacement = world.spawn(data);
            let mut physics = ScenePhysics2d::top_down().unwrap();
            if built {
                physics
                    .step(&mut world, extractor.components(), STEP)
                    .unwrap();
            }
            let before = world.get(owner).unwrap().components[kind].clone();
            run_call(
                &mut world,
                &extractor,
                &mut physics,
                "Physics.set_joint_endpoints(this.entity, World.find(\"Replacement\"), World.find(\"Body\"));",
                true,
            );
            let mut expected = before.clone();
            expected["first"] = json!("replacement");
            assert_eq!(world.get(owner).unwrap().components[kind], expected);
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 0);
            world.get_mut(replacement).unwrap().disabled = false;
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 1);
            run_call(
                &mut world,
                &extractor,
                &mut physics,
                "Physics.set_joint_endpoints(this.entity, null, World.find(\"Body\"));",
                true,
            );
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 0);
            physics
                .world_mut()
                .set_linear_velocity(moving, [3.0, 0.0])
                .unwrap();
            run_call(
                &mut world,
                &extractor,
                &mut physics,
                "Physics.set_joint_endpoints(this.entity, World.find(\"Anchor\"), World.find(\"Body\"));",
                true,
            );
            assert_eq!(world.get(owner).unwrap().components[kind], before);
            assert!(physics.world().linear_velocity(moving).unwrap()[0] > 2.9);
            physics
                .step(&mut world, extractor.components(), STEP)
                .unwrap();
            assert_eq!(physics.world().joint_count(), 1);
        }
    }
}

#[test]
fn invalid_second_endpoint_never_changes_the_first_or_settings() {
    for kind in KINDS {
        for problem in ["unstable", "same", "missing", "outside"] {
            let (extractor, mut world, owner, moving) = fixture(kind);
            let anchor = world
                .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
                .unwrap();
            world.get_mut(anchor).unwrap().name = Some("Anchor".into());
            world.get_mut(moving).unwrap().name = Some("Body".into());
            if problem == "unstable" {
                world.get_mut(moving).unwrap().source_id = None;
            }
            if problem == "outside" {
                let root = world.spawn(EntityData::default());
                world.set_parent(moving, Some(root)).unwrap();
            }
            let second = match problem {
                "same" => "Anchor",
                "missing" => "Missing",
                _ => "Body",
            };
            let before = world.get(owner).unwrap().components.clone();
            let mut physics = ScenePhysics2d::top_down().unwrap();
            let call = format!(
                "Physics.set_joint_endpoints(this.entity, World.find(\"Anchor\"), World.find(\"{second}\"));"
            );
            // A missing name is null, explicitly clearing the endpoint.
            run_call(
                &mut world,
                &extractor,
                &mut physics,
                &call,
                problem == "missing",
            );
            if problem == "missing" {
                assert_eq!(world.get(owner).unwrap().components[kind]["second"], "");
            } else {
                assert_eq!(world.get(owner).unwrap().components, before);
            }
        }
    }
}

fn run_call(
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
fn runtime_prefab_retargeting_uses_local_paths_and_rejects_other_instances() {
    for kind in KINDS {
        let (extractor, mut template, owner, body) = fixture(kind);
        let root = template
            .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
            .unwrap();
        template.get_mut(root).unwrap().name = Some("Anchor".into());
        template.get_mut(body).unwrap().name = Some("Body".into());
        template.set_parent(owner, Some(root)).unwrap();
        template.set_parent(body, Some(root)).unwrap();
        let prefab = sindri_core::PrefabDocument {
            entities: template.to_scene().unwrap().entities,
            ..sindri_core::PrefabDocument::default()
        };
        let mut world = World::default();
        let first = world.spawn_prefab(&prefab).unwrap();
        let second = world.spawn_prefab(&prefab).unwrap();
        let second_owner = second.by_source_id[&SceneEntityId::new("joint").unwrap()];
        // Only the first owner runs initially; named handles come from its spawn.
        world
            .get_mut(second_owner)
            .unwrap()
            .components
            .remove(ScriptComponent::TYPE_NAME);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        run_call(
            &mut world,
            &extractor,
            &mut physics,
            r#"Physics.set_joint_endpoints(this.entity, World.find("Anchor"), World.find("Body"));"#,
            true,
        );
        let first_owner = first.by_source_id[&SceneEntityId::new("joint").unwrap()];
        assert_eq!(
            world.get(first_owner).unwrap().components[kind]["second"],
            "body"
        );
        // Moving the script to the second owner cannot bind named handles in the first.
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
        run_call(
            &mut world,
            &extractor,
            &mut physics,
            r#"Physics.set_joint_endpoints(this.entity, World.find("Anchor"), World.find("Body"));"#,
            false,
        );
        assert_eq!(world.get(second_owner).unwrap().components, before);
    }
}

#[test]
fn stale_handles_and_missing_physics_fail_without_component_mutation() {
    for kind in KINDS {
        for stale in [false, true] {
            let (extractor, mut world, owner, body) = fixture(kind);
            world.get_mut(body).unwrap().name = Some("Body".into());
            let anchor = world
                .entity_for_source_id(&SceneEntityId::new("anchor").unwrap())
                .unwrap();
            world.get_mut(anchor).unwrap().name = Some("Anchor".into());
            let mut sources = ScriptSources::new();
            sources.insert("joint.decay", r#"script Joint {
                var target: Entity = null;
                fn start() { this.target = World.find("Body"); }
                fn update(dt: f32) { Physics.set_joint_endpoints(this.entity, World.find("Anchor"), this.target); }
            }"#);
            let input = InputState::default();
            let mut physics = ScenePhysics2d::top_down().unwrap();
            let mut scripts = Scripts::new();
            advance(
                &mut scripts,
                &mut world,
                &extractor,
                &sources,
                &input,
                &mut physics,
                true,
            );
            if stale {
                world.despawn_recursive(body).unwrap();
            }
            let before = world.get(owner).unwrap().components.clone();
            let frame = ScriptFrame::new(&sources, &input, 1.0 / 60.0);
            let frame = if stale {
                let (backend, events) = physics.for_scripts();
                frame.with_physics(Physics2d {
                    world: backend,
                    events,
                })
            } else {
                frame
            };
            let report = scripts.advance(&mut world, extractor.components(), frame);
            assert!(!report.failures.is_empty());
            assert_eq!(world.get(owner).unwrap().components, before);
        }
    }
}
