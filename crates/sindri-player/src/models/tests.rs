use super::*;
use sindri_assets::{AssetBytes, AssetDecoder, ModelAssetDecoder};
use sindri_core::{PrefabDocument, World};
use sindri_decay::ScriptSources;
use sindri_platform::InputState;

fn fixture(id: &str) -> (AssetId, ModelAsset) {
    let id = AssetId::new(id).unwrap();
    let asset = ModelAssetDecoder
        .decode(AssetBytes::new(
            id.clone(),
            include_bytes!("../../../sindri-assets/tests/fixtures/models/static-model.glb")
                .to_vec(),
        ))
        .unwrap();
    (id, asset)
}

#[test]
fn references_include_later_inactive_scenes_nested_prefabs_and_instance_overrides() {
    let mut prefabs = PrefabSources::new();
    prefabs.insert(
        "part.prefab",
        PrefabDocument::from_json(
            r#"{
        "format_version":1,"entities":[{"id":"root","components":{
        "sindri.physics3d.model_convex_collider":{"asset":"models/prefab.glb"}}}]}"#,
        )
        .unwrap(),
    );
    prefabs.insert(
        "nested.prefab",
        PrefabDocument::from_json(
            r#"{
        "format_version":1,"entities":[{"id":"nested","prefab":{"source":"part.prefab"}}]}"#,
        )
        .unwrap(),
    );
    let scenes = vec![
        ("empty.scene".into(), SceneDocument::default()),
        ("later.scene".into(), SceneDocument::from_json(r#"{
            "format_version":10,"entities":[
            {"id":"hidden","disabled":true,"components":{
              "sindri.physics3d.model_convex_collider":{"asset":"models/hidden.glb"}}},
            {"id":"placed","prefab":{"source":"nested.prefab","overrides":{
              "nested":{"components":{"sindri.physics3d.model_convex_collider":{"asset":"models/override.glb"}}}}}}
            ]}"#).unwrap()),
    ];
    let scene = SceneExtractor::new().unwrap();
    let ids = references(&scenes, &prefabs, scene.components()).unwrap();
    assert_eq!(
        ids.keys().map(AssetId::as_str).collect::<Vec<_>>(),
        [
            "models/hidden.glb",
            "models/override.glb",
            "models/prefab.glb"
        ]
    );
}

#[test]
fn collision_only_asset_is_bound_before_the_first_step_and_replays() {
    let document = SceneDocument::from_json(r#"{
        "format_version":10,"entities":[
        {"id":"level","components":{"sindri.physics3d.model_collider":{"asset":"models/static.glb"}}},
        {"id":"actor","transform_3d":{"position":[1.25,3.25,5]},"components":{
          "sindri.physics3d.character":{},"sindri.physics3d.collider":{"pieces":[{
            "shape":{"shape":"box","half_extents":[0.1,0.1,0.1]},
            "offset":[0,0,0],"rotation":[0,0,0,1],"sensor":false,
            "layers":{"memberships":1,"filter":4294967295},"friction":0.5,"restitution":0}]}}}] }"#).unwrap();
    let mut loaded = World::from_scene(&document).unwrap();
    let actor = loaded.entity_map[&sindri_core::SceneEntityId::new("actor").unwrap()];
    let mut scene = SceneExtractor::new().unwrap();
    let mut session = Session::with_sources(scene.components().clone(), ScriptSources::new());
    let refs = references(
        &[("main.scene".into(), document)],
        PrefabSources::none(),
        scene.components(),
    )
    .unwrap();
    install(
        vec![fixture("models/static.glb")],
        &refs,
        &mut scene,
        &mut session,
    )
    .unwrap();
    session
        .character_requests3d()
        .move_character(actor, [0.0, 0.0, -5.0], false)
        .unwrap();
    let checkpoint = session.checkpoint();
    let initial = loaded.world.clone();
    session
        .step(
            &mut loaded.world,
            &InputState::default(),
            (640.0, 360.0),
            0.01,
        )
        .unwrap();
    let pose = loaded.world.world_transform(actor).unwrap();
    assert!(
        pose.position[2] > 3.09 && pose.position[2] < 3.2,
        "{pose:?}"
    );
    session.restore(&checkpoint);
    loaded.world = initial;
    session
        .step(
            &mut loaded.world,
            &InputState::default(),
            (640.0, 360.0),
            0.01,
        )
        .unwrap();
    assert_eq!(loaded.world.world_transform(actor), Some(pose));
}

#[test]
fn only_authored_collision_assets_need_valid_physics_geometry() {
    let mut scene = SceneExtractor::new().unwrap();
    let mut session = Session::with_sources(scene.components().clone(), ScriptSources::new());
    let (id, mut asset) = fixture("models/empty.glb");
    // Empty render resources are legal, but have no physical triangles.
    asset.nodes.clear();
    asset.roots.clear();
    assert!(
        install(
            vec![(id.clone(), asset.clone())],
            &ModelCollisionReferences::new(),
            &mut scene,
            &mut session
        )
        .is_ok()
    );
    let error = install(
        vec![(id.clone(), asset)],
        &[(
            id.clone(),
            sindri_runtime::model_assets::ModelCollisionUse {
                triangles: true,
                convex: false,
            },
        )]
        .into_iter()
        .collect(),
        &mut scene,
        &mut session,
    )
    .unwrap_err();
    assert!(matches!(error, PlayerError::ModelCollision { asset, .. } if asset == id));
}

#[test]
fn missing_collision_asset_fails_installation_with_its_id() {
    let mut scene = SceneExtractor::new().unwrap();
    let mut session = Session::with_sources(scene.components().clone(), ScriptSources::new());
    let id = AssetId::new("models/missing.glb").unwrap();
    let error = install(
        Vec::new(),
        &[(
            id.clone(),
            sindri_runtime::model_assets::ModelCollisionUse {
                triangles: true,
                convex: false,
            },
        )]
        .into_iter()
        .collect(),
        &mut scene,
        &mut session,
    )
    .unwrap_err();
    assert!(matches!(error, PlayerError::MissingModelCollision(asset) if asset == id));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn model_collision_project_opens_compiles_blocks_and_walks_to_its_goal() {
    use sindri_core::LoadedScenes;
    use sindri_platform::{InputEvent, Key};

    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/model-collision");
    let project = crate::directory::read(&path).unwrap();
    let mut scene = sindri_runtime::scene_extractor().unwrap();
    let refs = references(&project.scenes, &project.prefabs, scene.components()).unwrap();
    let mut world = World::default();
    let mut scenes = LoadedScenes::new();
    let (name, document) = &project.scenes[0];
    scenes
        .enter_keeping_identities_with(&mut world, name, document, &project.prefabs)
        .unwrap();
    let mut session = Session::with_sources(scene.components().clone(), project.scripts)
        .with_prefabs(project.prefabs)
        .with_scenes(project.scenes, scenes);
    install(project.models, &refs, &mut scene, &mut session).unwrap();
    let mut input = InputState::default();
    let mut said = Vec::new();
    for (key, frames) in [(Key::D, 90), (Key::S, 45), (Key::D, 48), (Key::W, 45)] {
        input.apply(InputEvent::KeyPressed(key));
        for _ in 0..frames {
            let report = session
                .step(&mut world, &input, (960.0, 540.0), sindri_runtime::STEP)
                .unwrap();
            assert!(report.notes().is_empty(), "{report:?}");
            said.extend(report.scripts.printed.into_iter().map(|line| line.message));
        }
        input.apply(InputEvent::KeyReleased(key));
    }
    assert!(
        said.iter().any(|line| line == "Model wall blocked"),
        "{said:?}"
    );
    assert!(
        said.iter().any(|line| line == "Model prop goal reached"),
        "{said:?}"
    );
    assert!(
        said.iter().any(|line| line == "Model goal reached"),
        "{said:?}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_directory_loads_uppercase_glb_extensions_without_changing_asset_ids() {
    use std::fs;
    let root = std::env::temp_dir().join(format!("sindri-uppercase-model-{}", std::process::id()));
    fs::create_dir_all(root.join("models")).unwrap();
    fs::write(
        root.join("sindri.toml"),
        "format_version=1\n[project]\nname='Uppercase'\nmain_scene='main.scene'\n",
    )
    .unwrap();
    fs::write(root.join("main.scene"), r#"{"format_version":10,"entities":[{"id":"level","components":{"sindri.physics3d.model_collider":{"asset":"models/LEVEL.GLB"}}}]}"#).unwrap();
    fs::write(
        root.join("models/LEVEL.GLB"),
        include_bytes!("../../../sindri-assets/tests/fixtures/models/static-model.glb"),
    )
    .unwrap();
    let project = crate::directory::read(&root).unwrap();
    assert_eq!(project.models.len(), 1);
    assert_eq!(project.models[0].0.as_str(), "models/LEVEL.GLB");
    let mut scene = sindri_runtime::scene_extractor().unwrap();
    let collision = references(&project.scenes, &project.prefabs, scene.components()).unwrap();
    let mut loaded = World::from_scene(&project.scenes[0].1).unwrap();
    let mut session = Session::with_sources(scene.components().clone(), project.scripts);
    install(project.models, &collision, &mut scene, &mut session).unwrap();
    assert!(
        session
            .step(
                &mut loaded.world,
                &InputState::default(),
                (640.0, 360.0),
                sindri_runtime::STEP
            )
            .unwrap()
            .notes()
            .is_empty()
    );
    fs::remove_dir_all(root).unwrap();
}
