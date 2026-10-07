//! Typed contact values cross the host boundary as independent snapshots.

use std::time::Duration;

use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d};
use sindri_platform::InputState;
use sindri_scene::SceneExtractor;

#[test]
fn typed_contact_fields_copies_and_inactive_filtering_work_on_live_bodies() {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    let body = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({"source": "contacts.decay", "script": "Contacts"}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let floor = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld2d::new([0.0, -10.0]).unwrap();
    physics
        .insert_static_collider(
            floor,
            PhysicsPose2d::default(),
            &[Collider2d::rectangle([2.0, 0.5])],
        )
        .unwrap();
    physics
        .insert_body(
            body,
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [0.0, 1.0],
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::rectangle([0.5; 2])],
        )
        .unwrap();
    physics.step(Duration::from_millis(16)).unwrap();
    let expected = physics.contacts(body).unwrap();
    assert!(!expected.is_empty());
    let mut sources = ScriptSources::new();
    sources.insert("contacts.decay", r"
    script Contacts {
        fn update(dt: f32) {
            let contacts = Physics.contacts(this.entity);
            this.transform.position.x = contacts.length;
            if contacts.length > 0.0 {
                var contact = contacts[0.0];
                this.transform.position.y = contact.force.y;
                if contact.entity == this.entity || contact.normal.y < 0.9 || contact.point.y > 0.6 || contact.normal_impulse <= 0.0 || abs(contact.tangent_impulse) > 1.0 {
                    this.transform.position.z = 99.0;
                }
                contact.normal = Vec2(0.0, -1.0);
                let fresh = Physics.contacts(this.entity);
                if fresh[0.0].normal.y < 0.9 { this.transform.position.z = 99.0; }
            }
        }
    }");
    let mut scripts = Scripts::new();
    let input = InputState::default();
    for active in [true, false] {
        world.get_mut(floor).unwrap().disabled = !active;
        let report = scripts.advance(
            &mut world,
            extractor.components(),
            ScriptFrame::new(&sources, &input, 0.016).with_physics(Physics2d {
                world: &mut physics,
                events: &[],
            }),
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let position = world.get(body).unwrap().transform_3d.unwrap().position;
        assert!(
            position[2].abs() < f32::EPSILON,
            "all fields and copying work"
        );
        if active {
            assert!(position[0] >= 1.0);
            assert!(position[1] > 0.0);
        } else {
            assert!(position[0].abs() < f32::EPSILON);
        }
    }
    assert_eq!(
        expected,
        physics.contacts(body).unwrap(),
        "script edits leave physics alone"
    );
}

#[test]
fn spawn_window_is_empty_but_missing_body_and_physics_are_errors() {
    for (authored, enabled) in [(true, true), (false, true), (true, false)] {
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        let mut world = World::default();
        let entity = world.spawn(EntityData {
            transform_3d: Some(Transform3D::default()),
            components: [(
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({"source": "contacts.decay", "script": "Contacts"}),
            )]
            .into_iter()
            .collect(),
            ..EntityData::default()
        });
        if authored {
            world.get_mut(entity).unwrap().components.insert(
                "sindri.physics2d.collider".to_owned(),
                extractor
                    .components()
                    .default_payload("sindri.physics2d.collider")
                    .unwrap()
                    .clone(),
            );
        }
        let mut physics = PhysicsWorld2d::new([0.0; 2]).unwrap();
        let mut sources = ScriptSources::new();
        sources.insert("contacts.decay", "script Contacts { fn start() { this.transform.position.x = Physics.contacts(this.entity).length; } }");
        let input = InputState::default();
        let mut frame = ScriptFrame::new(&sources, &input, 0.016);
        if enabled {
            frame = frame.with_physics(Physics2d {
                world: &mut physics,
                events: &[],
            });
        }
        let report = Scripts::new().advance(&mut world, extractor.components(), frame);
        assert_eq!(
            report.failures.is_empty(),
            authored && enabled,
            "{:?}",
            report.failures
        );
    }
}
