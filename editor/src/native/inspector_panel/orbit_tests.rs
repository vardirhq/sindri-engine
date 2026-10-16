//! Orbit creation uses real scene targets, and settings survive checked edits.
use super::super::draft::{ProjectDefaults, SceneHolds, addable_components, component_default};
use super::{default_target, validate};
fn extractor() -> sindri_scene::SceneExtractor {
    super::super::super::scene_io::scene_extractor()
}
use serde_json::json;
use sindri_core::{
    CommandBuffer, CommandHistory, EntityData, EntityId, SceneEntityId, Transform3D, World,
    WorldCommand,
};

fn spawn(world: &mut World, name: &str) -> EntityId {
    world.spawn(EntityData {
        source_id: Some(SceneEntityId::new(name).unwrap()),
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    })
}
fn fixture() -> (World, EntityId, EntityId) {
    let mut world = World::default();
    let camera = spawn(&mut world, "camera");
    world.get_mut(camera).unwrap().components.insert(
        "sindri.camera".into(),
        extractor()
            .components()
            .default_payload("sindri.camera")
            .unwrap()
            .clone(),
    );
    let target = spawn(&mut world, "walker");
    (world, camera, target)
}
#[test]
fn orbit_is_offered_only_with_a_camera_and_a_real_scene_target() {
    let registry = extractor();
    let (world, camera, _) = fixture();
    let target = default_target(&world, camera).unwrap();
    assert_eq!(target, "walker");
    let project = ProjectDefaults {
        orbit_target: Some(&target),
        ..ProjectDefaults::default()
    };
    let offer = |present: &std::collections::BTreeMap<_, _>, project| {
        addable_components(
            registry.components(),
            present,
            project,
            SceneHolds::default(),
        )
        .into_iter()
        .find(|offer| offer.metadata.type_name == "sindri.camera.orbit")
        .unwrap()
        .withheld
    };
    assert!(offer(&world.get(camera).unwrap().components, project).is_none());
    assert!(
        offer(&std::collections::BTreeMap::default(), project)
            .unwrap()
            .contains("Camera")
    );
    assert!(
        offer(
            &world.get(camera).unwrap().components,
            ProjectDefaults::default()
        )
        .unwrap()
        .contains("stable ID")
    );
    let payload = component_default(registry.components(), "sindri.camera.orbit", project).unwrap();
    validate(&world, camera, &payload).unwrap();
    assert_eq!(payload["target"], json!("walker"));
    assert!(
        component_default(
            registry.components(),
            "sindri.camera.orbit",
            ProjectDefaults::default()
        )
        .is_none()
    );
}
#[test]
fn default_target_excludes_self_children_inactive_ui_and_unidentified_entities() {
    let (mut world, camera, target) = fixture();
    world.get_mut(target).unwrap().disabled = true;
    let child = spawn(&mut world, "child");
    world.set_parent(child, Some(camera)).unwrap();
    let ui = spawn(&mut world, "ui");
    world
        .get_mut(ui)
        .unwrap()
        .components
        .insert("sindri.ui.text".into(), json!({}));
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    });
    assert!(default_target(&world, camera).is_none());
    let valid = spawn(&mut world, "valid");
    assert_eq!(default_target(&world, camera).as_deref(), Some("valid"));
    world.get_mut(valid).unwrap().transform_3d = None;
    assert!(default_target(&world, camera).is_none());
}
#[test]
fn invalid_settings_and_target_edits_are_refused() {
    let (mut world, camera, target) = fixture();
    let payload = serde_json::to_value(sindri_scene::CameraOrbitComponent::new(
        SceneEntityId::new("walker").unwrap(),
    ))
    .unwrap();
    for (field, value) in [
        ("distance", json!(0.0)),
        ("pitch", json!(2.0)),
        ("collision_padding", json!(6.0)),
        ("smoothing", json!(-0.1)),
        ("target", json!("camera")),
        ("target", json!("missing")),
    ] {
        let mut edited = payload.clone();
        edited[field] = value;
        assert!(validate(&world, camera, &edited).is_err(), "{field}");
    }
    world.get_mut(target).unwrap().disabled = true;
    assert!(validate(&world, camera, &payload).is_err());
}
#[test]
fn checked_add_undo_redo_and_save_reopen_keep_orbit_fields() {
    let registry = extractor();
    let (mut world, camera, _) = fixture();
    let payload = component_default(
        registry.components(),
        "sindri.camera.orbit",
        ProjectDefaults {
            orbit_target: Some("walker"),
            ..ProjectDefaults::default()
        },
    )
    .unwrap();
    validate(&world, camera, &payload).unwrap();
    let mut buffer = CommandBuffer::new();
    buffer.push(WorldCommand::SetComponent {
        entity: camera,
        type_name: "sindri.camera.orbit".into(),
        payload: payload.clone(),
    });
    let mut history = CommandHistory::default();
    history
        .apply(buffer.into_transaction("Add orbit"), &mut world)
        .unwrap();
    history.undo(&mut world).unwrap();
    assert!(
        !world
            .get(camera)
            .unwrap()
            .components
            .contains_key("sindri.camera.orbit")
    );
    history.redo(&mut world).unwrap();
    let saved = world.to_scene().unwrap();
    let reopened = super::super::super::scene_io::load_document(&registry, &saved).unwrap();
    let restored = reopened
        .entity_for_source_id(&SceneEntityId::new("camera").unwrap())
        .unwrap();
    assert_eq!(
        reopened.get(restored).unwrap().components["sindri.camera.orbit"],
        payload
    );
}
