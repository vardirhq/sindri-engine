//! The typed ray result is a copied value, including after its entity is gone.
use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d};
use sindri_platform::InputState;

fn run(source: &str, host_physics: bool) -> (Scripts, World, Vec<String>) {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({"source":"ray.decay", "script":"Probe"}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let target = world.spawn(EntityData {
        name: Some("Target".to_owned()),
        ..EntityData::default()
    });
    let mut physics = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    physics
        .insert_static_collider(
            target,
            PhysicsPose2d {
                position: [3.0, 0.0],
                rotation: 0.0,
            },
            &[Collider2d::circle(1.0)],
        )
        .unwrap();
    let mut sources = ScriptSources::new();
    sources.insert("ray.decay", source);
    let input = InputState::default();
    let mut scripts = Scripts::new();
    let frame = ScriptFrame::new(&sources, &input, 1.0 / 60.0);
    let frame = if host_physics {
        frame.with_physics(Physics2d {
            world: &mut physics,
            events: &[],
        })
    } else {
        frame
    };
    let report = scripts.advance(&mut world, &registry, frame);
    let failures = report.failures.iter().map(ToString::to_string).collect();
    (scripts, world, failures)
}

#[test]
fn a_hit_can_be_read_copied_and_held_after_despawning_its_entity() {
    let (scripts, world, failures) = run(
        r#"
        script Probe {
            fn start() {
                let hit: RayHit2d = Physics.raycast(Vec2(0.0, 0.0), Vec2(10.0, 0.0), 5.0, 4294967295.0, false, null);
                if hit == null { return; }
                Game.set("entity", 0.0);
                if hit.entity == World.find("Target") { Game.set("entity", 1.0); }
                var copy = hit;
                copy.distance = 99.0;
                Game.set("distance", hit.distance);
                Game.set("point", hit.point.x);
                Game.set("normal", hit.normal.x);
                World.despawn(hit.entity);
                Game.set("snapshot", hit.distance);
                let miss = Physics.raycast(Vec2(0.0, 0.0), Vec2(1.0, 0.0), 5.0, 4294967295.0, false, null);
                if miss == null { Game.set("gone", 1.0); }
            }
        }
    "#,
        true,
    );
    assert!(failures.is_empty(), "{failures:?}");
    for (key, expected) in [
        ("entity", 1.0),
        ("distance", 2.0),
        ("point", 2.0),
        ("normal", -1.0),
        ("snapshot", 2.0),
        ("gone", 1.0),
    ] {
        assert!(
            (scripts.blackboard().get(key, 0.0) - expected).abs() < 0.0001,
            "{key}"
        );
    }
    assert!(
        world
            .entities()
            .all(|(_, data)| data.name.as_deref() != Some("Target"))
    );
}

#[test]
fn disabled_targets_and_explicit_exclusion_produce_null() {
    let (scripts, _, failures) = run(
        r#"
        script Probe {
            fn start() {
                let target = World.find("Target");
                let excluded = Physics.raycast(Vec2(0.0, 0.0), Vec2(1.0, 0.0), 5.0, 4294967295.0, false, target);
                if excluded == null { Game.set("excluded", 1.0); }
                World.set_active(target, false);
                let disabled = Physics.raycast(Vec2(0.0, 0.0), Vec2(1.0, 0.0), 5.0, 4294967295.0, false, null);
                if disabled == null { Game.set("disabled", 1.0); }
            }
        }
    "#,
        true,
    );
    assert!(failures.is_empty(), "{failures:?}");
    assert!((scripts.blackboard().get("excluded", 0.0) - 1.0).abs() < 0.0001);
    assert!((scripts.blackboard().get("disabled", 0.0) - 1.0).abs() < 0.0001);
}

#[test]
fn bad_masks_directions_and_missing_physics_report_the_call() {
    for (mask, direction, physics, message) in [
        ("-1.0", "1.0", true, "mask"),
        ("1.5", "1.0", true, "mask"),
        ("4294967296.0", "1.0", true, "mask"),
        ("1.0", "0.0", true, "ray_direction_length"),
        ("1.0", "1.0", false, "needs physics"),
    ] {
        let source = format!(
            "script Probe {{ fn start() {{ Physics.raycast(Vec2(0.0, 0.0), Vec2({direction}, 0.0), 5.0, {mask}, false, null); }} }}"
        );
        let (_, _, failures) = run(&source, physics);
        assert!(
            failures
                .iter()
                .any(|f| f.contains("Physics.raycast") && f.contains(message)),
            "{failures:?}"
        );
    }
}
