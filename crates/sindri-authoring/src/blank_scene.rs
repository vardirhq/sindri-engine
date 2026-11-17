//! What a brand-new scene contains, for every tool that makes one.
//!
//! One world camera, a sun, and a name. A scene with no camera is a legal
//! scene and a black Game view — the extract draws the player's view through
//! exactly one authored world camera — and "why is the game view empty" is
//! not the first question a new project should raise. The camera sits back
//! along +Z far enough to see the origin, as every shipped scene's does.
//!
//! The editor's New Scene and New Project and `sindri project new` all start
//! here, so a project begun from a shell is the project the editor would have
//! begun. Payloads come from the registry rather than from literals, for the
//! same reason every other component a tool creates does: a second copy of a
//! default is a copy that drifts.

use sindri_core::{
    ComponentSchemaRegistry, SceneComponent, SceneDocument, SceneEntity, SceneEntityId,
    SceneMetadata, Transform3D,
};
use sindri_scene::{CameraComponent, LightComponent, default_sun_transform};

/// A new scene called `name`.
#[must_use]
pub fn blank_scene(components: &ComponentSchemaRegistry, name: &str) -> SceneDocument {
    let mut camera =
        SceneEntity::new(SceneEntityId::new("world-camera").expect("a literal ID is not empty"));
    camera.name = Some("World Camera".to_owned());
    camera.transform_3d = Some(Transform3D {
        position: [0.0, 0.0, 9.0],
        ..Transform3D::default()
    });
    if let Some(payload) = components.default_payload(CameraComponent::TYPE_NAME) {
        camera
            .components
            .insert(CameraComponent::TYPE_NAME.to_owned(), payload.clone());
    }
    // And a sun, as a new scene in most engines has: anything 3D put in the
    // scene is lit from the first frame, and the light is an entity the author
    // can see and turn rather than a setting they have to find.
    let mut sun = SceneEntity::new(SceneEntityId::new("sun").expect("a literal ID is not empty"));
    sun.name = Some("Sun".to_owned());
    sun.transform_3d = Some(default_sun_transform());
    if let Some(payload) = components.default_payload(LightComponent::TYPE_NAME) {
        sun.components
            .insert(LightComponent::TYPE_NAME.to_owned(), payload.clone());
    }
    SceneDocument {
        metadata: SceneMetadata {
            name: Some(name.to_owned()),
            ..SceneMetadata::default()
        },
        entities: vec![sun, camera],
        ..SceneDocument::default()
    }
}
