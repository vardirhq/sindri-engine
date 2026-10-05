//! The physics components, and what a fresh one of each is.

use sindri_core::{AssetKind, ComponentSchemaRegistry, FieldMeaning};

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
    Ok(())
}
