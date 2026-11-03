//! The phone's authored village has the same ground as its Play session.

use std::path::Path;

use sindri_core::{SceneEntityId, World};
use sindri_mobile::OpenedScene;
use sindri_runtime::{ProjectRun, STEP};

fn sprites(world: &World, id: &str) -> Vec<Option<String>> {
    let source = SceneEntityId::new(id.to_owned()).expect("valid ID");
    let entity = world.entity_for_source_id(&source).expect("layer exists");
    let payload = &world.get(entity).expect("entity exists").components["sindri.tilemap"];
    let palette = payload["palette"].as_array().expect("palette");
    payload["tiles"].as_array().expect("tiles").iter().map(|tile| {
        tile.as_u64().map(|index| {
            let index = usize::try_from(index).expect("palette index fits");
            palette[index].as_str().expect("sprite name").to_owned()
        })
    }).collect()
}

#[test]
fn editor_sees_the_village_ground_before_play() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let editor = OpenedScene::open(&project, "village.scene").expect("editor opens village");
    let mut play = ProjectRun::open_scene(&project, "village.scene", [1080.0, 2400.0])
        .expect("Play opens village");
    for _ in 0..5 {
        assert!(play.step(STEP).expect("Play steps").notes().is_empty());
    }
    for id in ["ground", "soil", "cobble", "flag"] {
        let authored = sprites(&editor.world, id);
        assert!(authored.iter().any(Option::is_some), "{id} is not empty");
        assert_eq!(authored, sprites(&play.world, id), "{id} differs in Play");
    }
}
