use super::*;
use sindri_core::{SceneComponent, SceneDocument};
use sindri_scene::ModelCollider3dComponent;

#[test]
fn play_uses_unsaved_inactive_collision_references_and_refuses_missing_queue() {
    let components = sindri_runtime::scene_extractor()
        .unwrap()
        .components()
        .clone();
    let scripts = SceneScripts::for_scene(None);
    let tile_sets = TileSetBindings::new();
    let sources = PlaySources {
        components: &components,
        scripts: &scripts,
        tile_sets: &tile_sets,
        sheets: &[],
        models: None,
    };
    let document = SceneDocument::from_json(r#"{"format_version":10,"entities":[{"id":"level","disabled":true,"components":{"sindri.physics3d.model_collider":{"asset":"models/saved.glb"}}}]}"#).unwrap();
    let mut loaded = World::from_scene(&document).unwrap();
    let level = loaded.world.entities().next().unwrap().0;
    loaded.world.get_mut(level).unwrap().components.insert(
        ModelCollider3dComponent::TYPE_NAME.to_owned(),
        serde_json::json!({"asset":"models/unsaved.glb"}),
    );
    let wanted = collision_references(&sources, &loaded.world, OpenScene::default()).unwrap();
    assert_eq!(
        wanted
            .into_iter()
            .map(|id| id.as_str().to_owned())
            .collect::<Vec<_>>(),
        ["models/unsaved.glb"]
    );
    let error = start(
        &sources,
        &mut loaded.world,
        OpenScene::default(),
        Viewport {
            width: 640.0,
            height: 360.0,
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("models/unsaved.glb"), "{error}");
    loaded
        .world
        .get_mut(level)
        .unwrap()
        .components
        .remove(ModelCollider3dComponent::TYPE_NAME);
    assert!(
        start(
            &sources,
            &mut loaded.world,
            OpenScene::default(),
            Viewport {
                width: 640.0,
                height: 360.0
            }
        )
        .is_ok(),
        "removing the unsaved collider needs no model queue"
    );
}

#[test]
fn play_waits_for_the_queue_and_reports_decode_failures_with_the_asset_id() {
    use sindri_assets::{
        AssetBytes, AssetLoadQueueConfig, AssetLoader, MemoryAssetSource, ModelAssetDecoder,
    };
    use std::time::{Duration, Instant};

    let components = sindri_runtime::scene_extractor()
        .unwrap()
        .components()
        .clone();
    let scripts = SceneScripts::for_scene(None);
    let tile_sets = TileSetBindings::new();
    let document = SceneDocument::from_json(r#"{"format_version":10,"entities":[{"id":"level","components":{"sindri.physics3d.model_collider":{"asset":"models/queued.glb"}}}]}"#).unwrap();
    let mut loaded = World::from_scene(&document).unwrap();
    let id = AssetId::new("models/queued.glb").unwrap();
    let viewport = Viewport {
        width: 640.0,
        height: 360.0,
    };
    for bytes in [
        include_bytes!("../../../crates/sindri-assets/tests/fixtures/models/static-model.glb")
            .as_slice(),
        b"bad model".as_slice(),
    ] {
        let mut memory = MemoryAssetSource::default();
        memory.insert(AssetBytes::new(id.clone(), bytes.to_vec()));
        let mut model_queue = AssetLoader::new(
            memory,
            AssetLoadQueueConfig::unbounded(1),
            ModelAssetDecoder,
        )
        .unwrap();
        model_queue.request(id.clone()).unwrap();
        let sources = PlaySources {
            components: &components,
            scripts: &scripts,
            tile_sets: &tile_sets,
            sheets: &[],
            models: Some(&model_queue),
        };
        let error = start(&sources, &mut loaded.world, OpenScene::default(), viewport)
            .err()
            .unwrap();
        assert!(
            error.contains("models/queued.glb") && error.contains("still loading"),
            "{error}"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            model_queue.poll();
            if model_queue.outstanding() == 0 {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let sources = PlaySources {
            models: Some(&model_queue),
            components: &components,
            scripts: &scripts,
            tile_sets: &tile_sets,
            sheets: &[],
        };
        let result = start(&sources, &mut loaded.world, OpenScene::default(), viewport);
        if bytes == b"bad model" {
            let error = result.err().unwrap();
            assert!(
                error.contains("models/queued.glb") && !error.contains("still loading"),
                "{error}"
            );
        } else {
            let mut session = result.unwrap();
            let report = session
                .step(
                    &mut loaded.world,
                    &sindri_platform::InputState::default(),
                    (640.0, 360.0),
                    sindri_runtime::STEP,
                )
                .unwrap();
            assert!(report.notes().is_empty(), "{report:?}");
        }
    }
}
