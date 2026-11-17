//! Native project startup uses the same model queue/decoder as browser delivery.

use std::{
    error::Error,
    path::Path,
    time::{Duration, Instant},
};

use sindri_assets::{
    AssetLoadOutcome, AssetLoadQueueConfig, AssetLoader, FileSystemAssetSource, ModelAssetDecoder,
};
use sindri_core::AssetId;
use sindri_scene::SceneExtractor;

pub(super) fn bind(
    root: &Path,
    references: std::collections::BTreeSet<String>,
    scene: &mut SceneExtractor,
) -> Result<(), Box<dyn Error>> {
    let references: Vec<_> = references
        .into_iter()
        .map(AssetId::new)
        .collect::<Result<_, _>>()?;
    if references.is_empty() {
        return Ok(());
    }
    let defaults = AssetLoadQueueConfig::default();
    let config = AssetLoadQueueConfig::new(defaults.max_concurrent, references.len());
    let mut loader = AssetLoader::new(FileSystemAssetSource::new(root), config, ModelAssetDecoder)?;
    for id in &references {
        loader.request(id.clone())?;
    }
    let started = Instant::now();
    while loader.outstanding() != 0 {
        for outcome in loader.poll() {
            if let AssetLoadOutcome::Failed(error) = outcome {
                return Err(error.into());
            }
        }
        if started.elapsed() > Duration::from_secs(30) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "project model loading did not finish within 30 seconds",
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    for id in references {
        let asset = loader
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("model '{id}' completed without a value"))?;
        let prepared = sindri::model::prepare(asset)?;
        for warning in prepared.warnings {
            log::warn!("{id}: {warning}");
        }
        scene.bind_model(id, prepared.resource);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::{SceneDocument, World};
    use sindri_render::{FrameCommand, Viewport};
    use sindri_scene::{CameraView, TextureBindings};

    #[test]
    fn native_queue_binds_one_resource_for_repeated_scene_references() {
        let document = SceneDocument::from_json(
            r#"{
            "format_version":10, "entities":[
                {"id":"camera", "transform_3d":{"position":[0,0,10]},
                 "components":{"sindri.camera":{"projection":"orthographic","vertical_size":10,"near":0.1,"far":100}}},
                {"id":"one", "components":{"sindri.model":{"asset":"models/static-model.glb"}}},
                {"id":"two", "components":{"sindri.model":{"asset":"models/static-model.glb"}}}
            ]}"#,
        )
        .unwrap();
        let world = World::from_scene(&document).unwrap().world;
        let mut extractor = SceneExtractor::new().unwrap();
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/sindri-assets/tests/fixtures");
        bind(
            &root,
            sindri_scene::referenced_models(&world),
            &mut extractor,
        )
        .unwrap();
        let frame = extractor
            .extract(
                &world,
                Viewport::new(100, 100),
                CameraView::default(),
                &TextureBindings::new(),
            )
            .unwrap();
        let [one, two] = frame.passes() else {
            panic!("two model passes expected");
        };
        let (FrameCommand::Model { asset: first, .. }, FrameCommand::Model { asset: second, .. }) =
            (&one.command, &two.command)
        else {
            panic!("models expected");
        };
        assert!(std::sync::Arc::ptr_eq(first, second));
        assert_eq!(first.data().nodes.len(), 4);
    }
}
