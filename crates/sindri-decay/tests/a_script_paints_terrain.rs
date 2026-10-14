//! A script paints a terrain its map's sheet declares, and the neighbors whose
//! picture depends on the cell are redrawn with it.

use std::collections::BTreeMap;

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, SceneComponent, TerrainDocument, TerrainKind, Transform3D,
    World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{TileSetBindings, TilemapComponent};

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
        .register::<TilemapComponent>("Tilemap")
        .expect("sindri.tilemap registers");
    registry
}

fn run(source: &str) -> (World, sindri_decay::ScriptReport) {
    let mut world = World::default();
    world.spawn(EntityData {
        name: Some("Floor".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            TilemapComponent::TYPE_NAME.to_owned(),
            json!({ "texture": "textures/ground.png", "palette": [], "columns": 5,
                    "rows": 3, "tiles": vec![serde_json::Value::Null; 15] }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    world.spawn(EntityData {
        name: Some("Builder".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "builder.decay", "script": "Builder" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut tile_sets = TileSetBindings::new();
    tile_sets.bind_terrains(
        "textures/ground.sheet",
        BTreeMap::from([(
            "ground".to_owned(),
            TerrainDocument {
                kind: TerrainKind::Edges,
                sprites: (0..16).map(|mask| format!("g{mask}")).collect(),
            },
        )]),
    );
    let mut sources = ScriptSources::new();
    sources.insert("builder.decay", source);
    let report = Scripts::new().advance(
        &mut world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0).with_tile_sets(&tile_sets),
    );
    (world, report)
}

fn sprites(world: &World) -> Vec<Option<String>> {
    let map = world
        .entities()
        .find_map(|(_, data)| data.components.get(TilemapComponent::TYPE_NAME))
        .expect("the map is still there");
    map["tiles"]
        .as_array()
        .expect("tiles")
        .iter()
        .map(|tile| {
            tile.as_u64()
                .and_then(|index| map["palette"][usize::try_from(index).ok()?].as_str())
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn a_painted_strip_joins_up() {
    let (world, report) = run(r#"
        script Builder {
            fn update(dt: f32) {
                let floor = World.find("Floor");
                Grid.set_terrain(floor, 1.0, 1.0, "ground");
                Grid.set_terrain(floor, 2.0, 1.0, "ground");
                Grid.set_terrain(floor, 3.0, 1.0, "ground");
                Grid.set_terrain(floor, 3.0, 1.0, "");
            }
        }
        "#);
    assert!(report.failures.is_empty(), "{report:#?}");
    assert_eq!(
        sprites(&world)[5..10],
        [
            None,
            Some("g2".to_owned()),
            Some("g8".to_owned()),
            None,
            None
        ],
        "east-joined, then west-joined once the third cell is gone again"
    );
}

#[test]
fn an_unknown_terrain_names_itself() {
    let (_, report) = run(r#"
        script Builder {
            fn update(dt: f32) {
                Grid.set_terrain(World.find("Floor"), 0.0, 0.0, "lava");
            }
        }
        "#);
    assert!(format!("{report:?}").contains("lava"), "{report:#?}");
}
