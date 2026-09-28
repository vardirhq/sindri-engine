//! Structs across a project: declared in one file, built, kept, changed and
//! printed in another.

use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptReport, ScriptSources, Scripts};
use sindri_platform::InputState;

const CARDS: &str = "struct Card { name: String, weight: f32, at: Vec2 }";

const DEALER: &str = r#"script Dealer {
    var hand: List<Card> = [];
    var best = Card(name: "none", weight: 0.0, at: Vec2(0.0, 0.0));
    fn update(dt: f32) {
        hand.push(Card(weight: hand.length + 1.0, name: "card " + hand.length, at: Vec2(1.0, 2.0)));
        for card in hand {
            if card.weight > best.weight { best = card; }
        }
        best.at.x += 10.0;
        this.transform.position.x = best.weight;
        this.transform.position.y = best.at.x;
        print(best);
    }
}"#;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn frame(world: &mut World, scripts: &mut Scripts) -> ScriptReport {
    let mut sources = ScriptSources::new();
    sources.insert("cards.decay", CARDS);
    sources.insert("dealer.decay", DEALER);
    scripts.advance(
        world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    )
}

#[test]
fn a_struct_declared_in_one_file_is_used_in_another() {
    let mut world = World::default();
    let dealer = world.spawn(EntityData {
        name: Some("Dealer".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "dealer.decay", "script": "Dealer" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut scripts = Scripts::new();
    let first = frame(&mut world, &mut scripts);
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    let second = frame(&mut world, &mut scripts);
    assert!(second.failures.is_empty(), "{:?}", second.failures);
    let position = world
        .get(dealer)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position;
    assert!((position[0] - 2.0).abs() < 1e-6, "{position:?}");
    // The best card is the second one, copied into `best` and moved there.
    assert!((position[1] - 11.0).abs() < 1e-6, "{position:?}");
    assert_eq!(
        second.printed[0].message,
        r#"Card(name: "card 1", weight: 2, at: (11, 2))"#
    );
}
