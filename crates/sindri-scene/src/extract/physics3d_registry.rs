//! Defaults for the checked generic component authoring surface.

use sindri_core::{ComponentSchemaRegistry, FieldMeaning};
use sindri_physics::{Collider3d, RigidBody3d, RigidBodyKind};

use super::SceneExtractError;
use crate::{
    Collider3dComponent, PhysicsWorld3dComponent, RigidBody3dComponent, VoxelCollider3dComponent,
};

pub(super) fn register(components: &mut ComponentSchemaRegistry) -> Result<(), SceneExtractError> {
    components.register_with_default::<RigidBody3dComponent>(
        "Rigid Body 3D",
        serde_json::json!(RigidBody3d::default()),
    )?;
    components.describe::<RigidBody3dComponent>([(
        "kind",
        FieldMeaning::choice(RigidBodyKind::ALL.into_iter().map(RigidBodyKind::as_str)),
    )])?;
    components.register_with_default::<Collider3dComponent>(
        "Collider 3D",
        serde_json::json!({"pieces": [Collider3d::cuboid([0.5; 3])]}),
    )?;
    components.register_with_default::<VoxelCollider3dComponent>(
        "Voxel Collider 3D",
        serde_json::json!({
            "friction": 0.5,
            "restitution": 0.0,
            "layers": sindri_physics::CollisionLayers::ALL,
            "margin": 2.0,
        }),
    )?;
    components.register_with_default::<PhysicsWorld3dComponent>(
        "Physics 3D World",
        serde_json::json!({"gravity": [0.0, -9.81, 0.0], "layers": []}),
    )?;
    Ok(())
}
