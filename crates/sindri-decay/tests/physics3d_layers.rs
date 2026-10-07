//! Named masks remain dimension-specific and follow active authored settings.

#[path = "physics3d_controls/support.rs"]
mod support;

use decay_runtime::Value;
use serde_json::json;
use sindri_core::{EntityData, SceneComponent};
use sindri_scene::{PhysicsWorld2dComponent, PhysicsWorld3dComponent};
use support::{Fixture, near, reference};

fn text(name: &str) -> Value {
    Value::String(name.into())
}

#[test]
fn named_masks_select_real_3d_geometry_and_ignore_2d_settings() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("static", false);
    let settings = fixture.world.spawn(EntityData {
        components: [
            (
                PhysicsWorld3dComponent::TYPE_NAME.into(),
                json!({"layers": ["target", "other"]}),
            ),
            (
                PhysicsWorld2dComponent::TYPE_NAME.into(),
                json!({"layers": ["other", "target"]}),
            ),
        ]
        .into(),
        ..EntityData::default()
    });
    fixture.step();
    let mask = fixture
        .call(actor, "layer", &[text("target")], true)
        .unwrap();
    assert_eq!(mask, Value::Number(1.0));
    let args = [
        Value::Vec3([0.0, 0.0, -2.0]),
        Value::Vec3([0.0, 0.0, 1.0]),
        Value::Number(4.0),
        mask,
        Value::Bool(false),
        Value::Null,
    ];
    let Value::Struct { fields, .. } = fixture.call(actor, "raycast", &args, true).unwrap() else {
        panic!("named mask must hit the collider");
    };
    assert_eq!(fields[0], reference(actor));
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [0.0; 3],
    );
    assert_eq!(
        fixture
            .call(
                actor,
                "mask",
                &[Value::array(vec![
                    text("target"),
                    text("other"),
                    text("target")
                ])],
                true
            )
            .unwrap(),
        Value::Number(3.0)
    );
    fixture
        .world
        .get_mut(settings)
        .unwrap()
        .components
        .get_mut(PhysicsWorld3dComponent::TYPE_NAME)
        .unwrap()["layers"] = json!(["other", "target"]);
    assert_eq!(
        fixture
            .call(actor, "layer", &[text("target")], true)
            .unwrap(),
        Value::Number(2.0)
    );
    fixture.world.get_mut(settings).unwrap().disabled = true;
    assert!(
        fixture
            .call(actor, "layer", &[text("target")], true)
            .is_err()
    );
    assert_eq!(
        fixture
            .call(actor, "mask", &[Value::array(vec![])], true)
            .unwrap(),
        Value::Number(0.0)
    );
}

#[test]
fn layer_labels_retain_full_u32_bits_first_duplicate_and_empty_rules() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("static", false);
    let mut names: Vec<_> = (0..33).map(|index| format!("layer{index}")).collect();
    names[1] = names[0].clone();
    names[2] = String::new();
    fixture.world.spawn(EntityData {
        components: [(
            PhysicsWorld3dComponent::TYPE_NAME.into(),
            json!({"layers": names}),
        )]
        .into(),
        ..EntityData::default()
    });
    assert_eq!(
        fixture
            .call(actor, "layer", &[text("layer0")], true)
            .unwrap(),
        Value::Number(1.0)
    );
    assert_eq!(
        fixture
            .call(
                actor,
                "mask",
                &[Value::array(vec![text("layer0"), text("layer31")])],
                true
            )
            .unwrap(),
        Value::Number(2_147_483_649.0)
    );
    for name in ["", "layer32", "typo"] {
        assert!(fixture.call(actor, "layer", &[text(name)], true).is_err());
    }
}

#[test]
fn invalid_arguments_context_and_active_settings_fail_explicitly() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("static", false);
    for (call, args) in [
        ("layer", vec![]),
        ("layer", vec![text("a"), text("b")]),
        ("layer", vec![Value::Number(1.0)]),
        ("mask", vec![text("a")]),
        ("mask", vec![Value::array(vec![Value::Null])]),
        ("mask", vec![Value::array(vec![text("unknown")])]),
    ] {
        assert!(fixture.call(actor, call, &args, true).is_err());
    }
    assert!(
        fixture
            .call(actor, "mask", &[Value::array(vec![])], false)
            .unwrap_err()
            .contains("no 3D physics")
    );
    let settings = fixture.world.spawn(EntityData {
        components: [(
            PhysicsWorld3dComponent::TYPE_NAME.into(),
            json!({"layers": 3}),
        )]
        .into(),
        ..EntityData::default()
    });
    assert!(
        fixture
            .call(actor, "mask", &[Value::array(vec![])], true)
            .unwrap_err()
            .contains("invalid 3D world")
    );
    fixture.world.get_mut(settings).unwrap().components.insert(
        PhysicsWorld3dComponent::TYPE_NAME.into(),
        json!({"layers": ["a"]}),
    );
    fixture.world.spawn(EntityData {
        components: [(PhysicsWorld3dComponent::TYPE_NAME.into(), json!({}))].into(),
        ..EntityData::default()
    });
    assert!(
        fixture
            .call(actor, "layer", &[text("a")], true)
            .unwrap_err()
            .contains("multiple active")
    );
}
