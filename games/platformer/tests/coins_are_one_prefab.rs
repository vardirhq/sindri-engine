//! The level's coins are placed as instances of one prefab.
//!
//! Ten coins used to be ten copies of the same sprite, clip, collider and tag,
//! and changing how a coin spins meant changing it ten times. They are now ten
//! places in the scene and one `prefabs/coin.prefab`, which is what makes a
//! change to a coin a change to every coin.

use platformer::Run;
use sindri_core::{SceneComponent, SceneDocument, TagsComponent};

#[test]
fn every_coin_is_an_instance_of_the_coin_prefab() {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/platformer.scene"),
    )
    .expect("the scene reads");
    let scene = SceneDocument::from_json(&text).expect("the scene parses");
    let coins: Vec<_> = scene
        .entities
        .iter()
        .filter(|entity| entity.id.as_str().starts_with("coin-"))
        .collect();
    assert_eq!(coins.len(), 10);
    for coin in coins {
        let instance = coin.prefab.as_ref().expect("placed as an instance");
        assert_eq!(instance.source, "prefabs/coin.prefab");
        assert!(
            coin.components.is_empty(),
            "made of the prefab's components"
        );
    }
}

#[test]
fn the_instances_are_coins_when_the_game_runs() {
    let run = Run::open().expect("the project opens");
    let tagged = run
        .world
        .entities()
        .filter(|(_, data)| {
            data.components
                .get(TagsComponent::TYPE_NAME)
                .and_then(|tags| serde_json::from_value::<TagsComponent>(tags.clone()).ok())
                .is_some_and(|tags| tags.has("coin"))
        })
        .count();
    assert_eq!(tagged, 10);
}
