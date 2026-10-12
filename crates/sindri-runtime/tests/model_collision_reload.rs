//! A shared static/convex asset is prepared atomically before host replacement.
use serde_json::json;
use sindri_assets::{AssetBytes, AssetDecoder, ModelAssetDecoder};
use sindri_core::{AssetId, SceneDocument, World};
use sindri_decay::{PrefabSources, ScriptSources};
use sindri_platform::InputState;
use sindri_runtime::{
    Session,
    model_assets::{
        ModelCollisionUse, collision_references, collision_world_references, merge_references,
    },
};
use sindri_scene::SceneExtractor;

fn document(kind: &str) -> SceneDocument {
    SceneDocument::from_json(&json!({"format_version":10,"entities":[{"id":"prop","components":{kind:{"asset":"models/prop.glb"}}}]}).to_string()).unwrap()
}
#[test]
fn unsaved_and_later_scene_uses_merge_without_losing_either_representation() {
    let components = SceneExtractor::new().unwrap().components().clone();
    let world = World::from_scene(&document("sindri.physics3d.model_convex_collider"))
        .unwrap()
        .world;
    let mut wanted = collision_references(
        &[(
            "later.scene".into(),
            document("sindri.physics3d.model_collider"),
        )],
        PrefabSources::none(),
        &components,
    )
    .unwrap();
    merge_references(
        &mut wanted,
        collision_world_references(&world, &components).unwrap(),
    );
    assert_eq!(wanted.len(), 1);
    assert_eq!(
        *wanted.values().next().unwrap(),
        ModelCollisionUse {
            triangles: true,
            convex: true
        }
    );
}
// The filter belongs to physics; this runtime test stays at the scene seam.
#[allow(clippy::default_trait_access)]
fn height(session: &Session, x: f32) -> f32 {
    session
        .physics3d()
        .world()
        .raycast([x, 10.0, 0.0], [0.0, -1.0, 0.0], 20.0, Default::default())
        .unwrap()
        .unwrap()
        .point[1]
}
#[test]
fn invalid_convex_reload_retains_both_old_bindings() {
    let id = AssetId::new("models/prop.glb").unwrap();
    let doc = SceneDocument::from_json(r#"{"format_version":10,"entities":[{"id":"mesh","components":{"sindri.physics3d.model_collider":{"asset":"models/prop.glb"}}},{"id":"hull","transform_3d":{"position":[3,0,0]},"components":{"sindri.physics3d.model_convex_collider":{"asset":"models/prop.glb"}}}]}"#).unwrap();
    let mut world = World::from_scene(&doc).unwrap().world;
    let components = SceneExtractor::new().unwrap().components().clone();
    let mut session = Session::with_sources(components, ScriptSources::new());
    let usage = ModelCollisionUse {
        triangles: true,
        convex: true,
    };
    let cube = ModelAssetDecoder
        .decode(AssetBytes::new(
            id.clone(),
            include_bytes!("../../../examples/model-collision/assets/models/prop.glb").to_vec(),
        ))
        .unwrap();
    session
        .bind_model_collisions(id.clone(), &cube, usage)
        .unwrap();
    session
        .step(&mut world, &InputState::default(), (640.0, 360.0), 0.01)
        .unwrap();
    assert!((height(&session, 0.0) - 0.5).abs() < 0.001);
    assert!((height(&session, 3.0) - 0.5).abs() < 0.001);
    let planar = ModelAssetDecoder
        .decode(AssetBytes::new(
            id.clone(),
            include_bytes!("../../sindri-assets/tests/fixtures/models/static-model.glb").to_vec(),
        ))
        .unwrap();
    assert!(session.bind_model_collisions(id, &planar, usage).is_err());
    session
        .step(&mut world, &InputState::default(), (640.0, 360.0), 0.01)
        .unwrap();
    assert!((height(&session, 0.0) - 0.5).abs() < 0.001);
    assert!((height(&session, 3.0) - 0.5).abs() < 0.001);
}
