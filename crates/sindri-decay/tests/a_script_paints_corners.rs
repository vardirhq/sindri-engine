//! A script paints one corner of a corners terrain its map's sheet declares,
//! and the tiles that meet at the corner are redrawn with it.

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
            "dirt".to_owned(),
            TerrainDocument {
                kind: TerrainKind::Corners,
                sprites: (0..16).map(|mask| format!("d{mask}")).collect(),
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
fn two_corners_draw_one_shared_edge() {
    let (world, report) = run(r#"
        script Builder {
            fn update(dt: f32) {
                let floor = World.find("Floor");
                Grid.set_terrain_corner(floor, 1.0, 1.0, "dirt", true);
                Grid.set_terrain_corner(floor, 1.0, 2.0, "dirt", true);
                Grid.set_terrain_corner(floor, 1.0, 2.0, "dirt", false);
            }
        }
        "#);
    assert!(report.failures.is_empty(), "{report:#?}");
    let tiles = sprites(&world);
    // Row 1 of five columns: the tile west of the corner sees its east side,
    // the tile east of it its west side; the second corner was taken back.
    assert_eq!(tiles[5].as_deref(), Some("d4"), "north-east of the tile");
    assert_eq!(tiles[6].as_deref(), Some("d8"), "north-west of the tile");
    assert_eq!(
        tiles[10].as_deref(),
        Some("d0"),
        "the cleared corner left the tile as plain ground"
    );
}

#[test]
fn a_cell_terrain_and_a_corner_call_do_not_mix() {
    let (_, report) = run(r#"
        script Builder {
            fn update(dt: f32) {
                Grid.set_terrain(World.find("Floor"), 0.0, 0.0, "dirt");
            }
        }
        "#);
    assert!(format!("{report:?}").contains("by corner"), "{report:#?}");
}
