use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D};

use super::*;
use crate::{Collider2dComponent, RigidBody2dComponent, SceneExtractor, ScenePhysics2d};

fn profile(values: &serde_json::Value) -> ProfileDocument {
    ProfileDocument::from_json(
        &json!({"format_version": 1, "type": "physics_material", "values": values}).to_string(),
    )
    .unwrap()
}

#[test]
fn shared_validation_rejects_typos_ranges_and_missing_coefficients() {
    for values in [
        json!({"friction": -1.0, "restitution": 0.0}),
        json!({"friction": 0.2, "restitution": 1.1}),
        json!({"friction": "0.2", "restitution": 0.0}),
        json!({"friction": 0.2}),
        json!({"friction": 0.2, "restitution": 0.0, "bounciness": 0.8}),
        json!({"friction": 1e100, "restitution": 0.0}),
    ] {
        let error = physics_material_profile("wood.profile", &profile(&values)).unwrap_err();
        assert!(error.to_string().contains("wood.profile"));
    }
    assert!(
        physics_material_profile("game.profile", &ProfileDocument::default())
            .unwrap()
            .is_none()
    );
    assert!(
        physics_material_profile(
            "wood.profile",
            &profile(&json!({"friction": 2.0, "restitution": 1.0}))
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn literals_profile_and_explicit_overrides_have_unambiguous_precedence() {
    let document = profile(&json!({"friction": 0.8, "restitution": 0.7}));
    let sources = PhysicsMaterialSources::from_profiles([("wood.profile", &document)]).unwrap();
    let mut pieces = [Collider2d::circle(1.0), Collider2d::rectangle([1.0, 1.0])];
    pieces[0].friction = 0.1;
    let mut component: PhysicsMaterial2dComponent = serde_json::from_value(json!({})).unwrap();
    sources.apply(&component, &mut pieces).unwrap();
    assert!((pieces[0].friction - 0.1).abs() < f32::EPSILON);
    component.profile = "wood.profile".into();
    component.override_restitution = true;
    component.restitution = 0.2;
    sources.apply(&component, &mut pieces).unwrap();
    for piece in pieces {
        assert!((piece.friction - 0.8).abs() < f32::EPSILON);
        assert!((piece.restitution - 0.2).abs() < f32::EPSILON);
    }
    component.profile = "missing.profile".into();
    assert!(sources.apply(&component, &mut pieces).is_err());
}

#[test]
fn reload_changes_registered_coefficients_without_resetting_motion() {
    let components = SceneExtractor::new().unwrap().components().clone();
    let mut world = World::default();
    let pieces = [
        Collider2d::circle(1.0),
        Collider2d {
            sensor: true,
            ..Collider2d::circle(0.5)
        },
    ];
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                Collider2dComponent::TYPE_NAME.into(),
                json!({"pieces": pieces}),
            ),
            (
                RigidBody2dComponent::TYPE_NAME.into(),
                serde_json::to_value(sindri_physics::RigidBody2d::default()).unwrap(),
            ),
            (
                PhysicsMaterial2dComponent::TYPE_NAME.into(),
                json!({"profile": "wood.profile"}),
            ),
        ]
        .into(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let step = std::time::Duration::from_millis(16);
    assert!(matches!(
        physics.step(&mut world, &components, step),
        Err(PhysicsSyncError::Material(PhysicsMaterialError::Missing(_)))
    ));
    let old = profile(&json!({"friction": 0.4, "restitution": 0.1}));
    physics.set_materials(PhysicsMaterialSources::from_profiles([("wood.profile", &old)]).unwrap());
    physics.step(&mut world, &components, step).unwrap();
    physics
        .world_mut()
        .set_linear_velocity(entity, [2.0, 0.0])
        .unwrap();
    let new = profile(&json!({"friction": 0.0, "restitution": 0.9}));
    physics.set_materials(PhysicsMaterialSources::from_profiles([("wood.profile", &new)]).unwrap());
    physics.step(&mut world, &components, step).unwrap();
    assert!((physics.world().linear_velocity(entity).unwrap()[0] - 2.0).abs() < 1e-4);
    for material in physics.world().materials(entity).unwrap() {
        assert!(material.friction.abs() < f32::EPSILON);
        assert!((material.restitution - 0.9).abs() < f32::EPSILON);
    }
    assert_eq!(
        referenced_physics_materials(&world, &components).unwrap(),
        ["wood.profile"]
    );
}
