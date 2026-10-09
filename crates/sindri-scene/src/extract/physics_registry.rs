//! The physics components, and what a fresh one of each is.

use sindri_core::{AssetKind, ComponentSchemaRegistry, FieldMeaning};
use sindri_physics::MotorMode2d;

use crate::physics::{
    Collider2dComponent, OneWay2dComponent, PhysicsWorld2dComponent, RigidBody2dComponent,
};
use crate::tilemap_collision::TilemapCollider2dComponent;

use super::SceneExtractError;

pub(super) fn register(components: &mut ComponentSchemaRegistry) -> Result<(), SceneExtractError> {
    // Physics defaults are ordinary Sindri values rather than backend
    // values. A newly added body starts dynamic and a collider starts as a
    // one-unit box, so both are immediately valid and visible in the
    // generic command-backed inspector.
    components.register_with_default::<RigidBody2dComponent>(
        "Rigid Body 2D",
        serde_json::json!({
            "kind": "dynamic",
            "pose": { "position": [0.0, 0.0], "rotation": 0.0 },
            "linear_velocity": [0.0, 0.0],
            "angular_velocity": 0.0,
            "gravity_scale": 1.0,
            "linear_damping": 0.0,
            "angular_damping": 0.0,
            "lock_rotation": false,
            "continuous_collision": false
        }),
    )?;
    // The default is written as a compound of one rather than as a bare
    // collider: both parse, and this is the shape a second piece is added to.
    // A default in the older single form would make every new collider need
    // rewriting before it could grow.
    components.register_with_default::<Collider2dComponent>(
        "Collider 2D",
        serde_json::json!({
            "pieces": [{
                "shape": { "shape": "box", "half_extents": [0.5, 0.5] },
                "offset": [0.0, 0.0],
                "rotation": 0.0,
                "sensor": false,
                "layers": { "memberships": 4_294_967_295_u32, "filter": 4_294_967_295_u32 },
                "friction": 0.5,
                "restitution": 0.0
            }]
        }),
    )?;
    // Every painted tile solid, as a tilemap collider added to a level is
    // meant to make it: passable sprites are the exception an author names.
    components.register_with_default::<TilemapCollider2dComponent>(
        "Tilemap Collider 2D",
        serde_json::json!({
            "passable": [],
            "layers": { "memberships": 4_294_967_295_u32, "filter": 4_294_967_295_u32 },
            "friction": 0.5,
            "restitution": 0.0
        }),
    )?;
    // Down at Earth's pull: the component is added to make things fall, and
    // a scene seen from above simply does not carry one.
    components.register_with_default::<PhysicsWorld2dComponent>(
        "Physics 2D World",
        serde_json::json!({ "gravity": [0.0, -9.81], "layers": [] }),
    )?;
    components.register_with_default::<OneWay2dComponent>(
        "One-Way Platform 2D",
        serde_json::json!({ "normal": [0.0, 1.0], "angle": std::f32::consts::FRAC_PI_4 }),
    )?;
    components.register_with_default::<crate::PhysicsMaterial2dComponent>(
        "Physics Material 2D",
        serde_json::json!({"profile": "", "override_friction": false, "friction": 0.5, "override_restitution": false, "restitution": 0.0}),
    )?;
    components.describe::<crate::PhysicsMaterial2dComponent>([(
        "profile",
        FieldMeaning::Asset(AssetKind::Profile),
    )])?;
    components.register_with_default::<crate::DistanceJoint2dComponent>(
        "Distance Joint 2D",
        serde_json::json!({"first": "", "second": "", "enabled": true, "max_distance": 1.0}),
    )?;
    components.describe::<crate::DistanceJoint2dComponent>([
        ("first", FieldMeaning::Entity),
        ("second", FieldMeaning::Entity),
    ])?;
    components.register_with_default::<crate::Character2dComponent>(
        "Character 2D",
        serde_json::json!({"skin": 0.01, "max_iterations": 8, "up": [0.0, 1.0],
            "max_slope_angle": std::f32::consts::FRAC_PI_4, "snap_distance": 0.0,
            "step_height": 0.0, "carry_platforms": true}),
    )?;
    components
        .describe::<crate::Character2dComponent>([("max_slope_angle", FieldMeaning::Angle)])?;
    components.describe::<OneWay2dComponent>([("angle", FieldMeaning::Angle)])?;
    super::physics3d_registry::register(components)?;
    register_hinge(components)?;
    register_linear_joints(components)?;
    Ok(())
}

fn register_hinge(components: &mut ComponentSchemaRegistry) -> Result<(), SceneExtractError> {
    components.register_with_default::<crate::HingeJoint2dComponent>(
        "Hinge Joint 2D",
        serde_json::json!({
            "first": "", "second": "", "enabled": true, "first_anchor": [0.0, 0.0],
            "second_anchor": [0.0, 0.0], "limits_enabled": false,
            "lower_angle": 0.0, "upper_angle": 0.0, "motor_enabled": false,
            "motor_velocity": 0.0, "motor_max_torque": 0.0, "motor_mode": "velocity",
            "motor_target_angle": 0.0, "motor_stiffness": 0.0, "motor_damping": 0.0
        }),
    )?;
    components.describe::<crate::HingeJoint2dComponent>([
        ("first", FieldMeaning::Entity),
        ("second", FieldMeaning::Entity),
        ("motor_mode", motor_modes()),
        ("lower_angle", FieldMeaning::Angle),
        ("upper_angle", FieldMeaning::Angle),
        ("motor_target_angle", FieldMeaning::Angle),
    ])?;
    Ok(())
}

fn register_linear_joints(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    components.register_with_default::<crate::SliderJoint2dComponent>(
        "Slider Joint 2D",
        serde_json::json!({"first": "", "second": "", "enabled": true, "first_anchor": [0.0, 0.0],
            "second_anchor": [0.0, 0.0], "first_axis": [1.0, 0.0], "second_axis": [1.0, 0.0],
            "limits_enabled": false, "lower_distance": 0.0, "upper_distance": 0.0,
            "motor_enabled": false, "motor_velocity": 0.0, "motor_max_force": 0.0,
            "motor_mode": "velocity", "motor_target_distance": 0.0,
            "motor_stiffness": 0.0, "motor_damping": 0.0}),
    )?;
    components.describe::<crate::SliderJoint2dComponent>([
        ("first", FieldMeaning::Entity),
        ("second", FieldMeaning::Entity),
        ("motor_mode", motor_modes()),
    ])?;
    components.register_with_default::<crate::SpringJoint2dComponent>(
        "Spring Joint 2D",
        serde_json::json!({"first": "", "second": "", "enabled": true, "first_anchor": [0.0, 0.0],
            "second_anchor": [0.0, 0.0], "rest_length": 1.0, "stiffness": 10.0, "damping": 1.0}),
    )?;
    components.describe::<crate::SpringJoint2dComponent>([
        ("first", FieldMeaning::Entity),
        ("second", FieldMeaning::Entity),
    ])?;
    Ok(())
}

fn motor_modes() -> FieldMeaning {
    FieldMeaning::choice(MotorMode2d::ALL.into_iter().map(MotorMode2d::as_str))
}
