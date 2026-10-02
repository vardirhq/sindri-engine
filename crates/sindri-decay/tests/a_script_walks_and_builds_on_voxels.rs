//! A grid whose ground is a voxel world, from a script: reading a block,
//! building and taking one away, and walking across what was generated.
//!
//! The same `Grid` calls a stacked volume answers, because to a script a
//! cell is a column, a row and a level whichever way the ground is stored.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, SceneEntityId, TileSetDocument,
    Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{TileSetBindings, VoxelGround, VoxelWorldComponent};

/// The flat world's ground is at this level everywhere.
const GROUND: i32 = 4;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn tile_sets() -> TileSetBindings {
    let document = TileSetDocument::from_json(
        r#"{ "format_version": 1, "tiles": {
             "grass": { "tags": ["soft"],
                        "faces": { "top": { "sprite": "b.png#0", "size": [1.0, 1.0] } } },
             "stone": { "faces": { "top": { "sprite": "b.png#0", "size": [1.0, 1.0] } } }
           } }"#,
    )
    .expect("the tile set decodes");
    let mut bindings = TileSetBindings::new();
    bindings.bind("world.tileset", document).unwrap();
    bindings
}

fn voxel_world() -> serde_json::Value {
    json!({
        "blocks": "world.tileset",
        "generator": {
            "kind": "layered_terrain",
            "base_height": GROUND,
            "height_variation": 0,
            "surface_voxel": "grass",
            "subsurface_voxel": "stone",
            "deep_voxel": "stone"
        }
    })
}

fn spawn(world: &mut World, id: &str, name: &str, components: &serde_json::Value) -> EntityId {
    world.spawn(EntityData {
        name: Some(name.to_owned()),
        source_id: Some(SceneEntityId::new(id).expect("a stable id")),
        transform_3d: Some(Transform3D::default()),
        components: components
            .as_object()
            .expect("components are an object")
            .clone()
            .into_iter()
            .collect(),
        ..EntityData::default()
    })
}

fn floor(world: &mut World) -> EntityId {
    spawn(
        world,
        "floor",
        "Floor",
        &json!({
            "sindri.tile_grid": {
                "columns": 64, "rows": 64, "cell_size": [1.0, 1.0],
                "cell_height": 1.0, "space": "solid"
            },
            "sindri.grid.navigation": { "max_step": 1.0 },
            "sindri.voxel_world": voxel_world()
        }),
    )
}

fn scripted(world: &mut World, script: &str) -> ScriptSources {
    spawn(
        world,
        "actor",
        "Actor",
        &json!({ ScriptComponent::TYPE_NAME: { "source": "voxels.decay", "script": "Actor" } }),
    );
    let mut sources = ScriptSources::new();
    sources.insert("voxels.decay", script);
    sources
}

fn run(world: &mut World, sources: &ScriptSources) -> Vec<ScriptFailure> {
    let sets = tile_sets();
    let input = InputState::default();
    let frame = ScriptFrame::new(sources, &input, 1.0 / 60.0).with_tile_sets(&sets);
    Scripts::new().advance(world, &registry(), frame).failures
}

fn ground(world: &World, floor: EntityId) -> VoxelGround {
    let component: VoxelWorldComponent = serde_json::from_value(
        world.get(floor).unwrap().components[VoxelWorldComponent::TYPE_NAME].clone(),
    )
    .unwrap();
    VoxelGround::of(&component, Some(&tile_sets())).expect("the world reads")
}

fn edits(world: &World, floor: EntityId) -> serde_json::Value {
    world.get(floor).unwrap().components[VoxelWorldComponent::TYPE_NAME]
        .get("edits")
        .cloned()
        .unwrap_or_else(|| json!([]))
}

#[test]
fn a_script_reads_generated_blocks_by_grid_cell() {
    let mut world = World::default();
    let floor = floor(&mut world);
    assert_eq!(
        ground(&world, floor).surface(3, 5).map(|top| top.0),
        Some(GROUND)
    );
    let sources = scripted(
        &mut world,
        &format!(
            r#"
            script Actor {{
                fn update(dt: f32) {{
                    let floor = World.find("Floor");
                    if Grid.block(floor, 3.0, 5.0, {GROUND}.0) == "grass" {{
                        this.transform.scale.x = 2.0;
                    }}
                    if Grid.block(floor, 3.0, 5.0, {above}.0) == "" {{
                        this.transform.scale.y = 3.0;
                    }}
                    if Grid.tagged(floor, 3.0, 5.0, {GROUND}.0, "soft") {{
                        this.transform.scale.z = 4.0;
                    }}
                }}
            }}
            "#,
            above = GROUND + 1
        ),
    );
    assert!(run(&mut world, &sources).is_empty());
    let actor = world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Actor"))
        .unwrap()
        .0;
    let scale = world.get(actor).unwrap().transform_3d.unwrap().scale;
    assert!(
        scale
            .iter()
            .zip([2.0, 3.0, 4.0])
            .all(|(got, want)| (got - want).abs() < f32::EPSILON),
        "every answer was the one expected: {scale:?}"
    );
}

#[test]
fn building_and_taking_away_is_kept_as_edits_and_nothing_else() {
    let mut world = World::default();
    let floor = floor(&mut world);
    let sources = scripted(
        &mut world,
        &format!(
            r#"
            script Actor {{
                var built: bool = false;
                fn update(dt: f32) {{
                    let floor = World.find("Floor");
                    if !built {{
                        Grid.set_block(floor, 2.0, 2.0, {above}.0, "stone");
                        Grid.set_block(floor, 7.0, 1.0, {GROUND}.0, "");
                        built = true;
                    }} else {{
                        Grid.set_block(floor, 2.0, 2.0, {above}.0, "");
                        Grid.set_block(floor, 7.0, 1.0, {GROUND}.0, "grass");
                    }}
                }}
            }}
            "#,
            above = GROUND + 1
        ),
    );
    let mut scripts = Scripts::new();
    let sets = tile_sets();
    let input = InputState::default();
    let failures = scripts
        .advance(
            &mut world,
            &registry(),
            ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_tile_sets(&sets),
        )
        .failures;
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        edits(&world, floor),
        json!([
            { "at": [2, GROUND + 1, 2], "block": "stone" },
            { "at": [7, GROUND, 1], "block": "" }
        ])
    );
    let built = ground(&world, floor);
    assert_eq!(built.surface(2, 2).map(|top| top.0), Some(GROUND + 1));
    assert_eq!(built.surface(7, 1).map(|top| top.0), Some(GROUND - 1));

    // Putting both back as they were generated leaves no edit at all.
    let failures = scripts
        .advance(
            &mut world,
            &registry(),
            ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_tile_sets(&sets),
        )
        .failures;
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(edits(&world, floor), json!([]));
}

#[test]
fn a_block_the_set_does_not_define_is_refused_where_it_is_written() {
    let mut world = World::default();
    floor(&mut world);
    let sources = scripted(
        &mut world,
        r#"
        script Actor {
            fn update(dt: f32) {
                Grid.set_block(World.find("Floor"), 1.0, 1.0, 9.0, "marble");
            }
        }
        "#,
    );
    let failures = run(&mut world, &sources);
    assert_eq!(failures.len(), 1);
    assert!(
        failures[0].to_string().contains("marble"),
        "{}",
        failures[0]
    );
}

/// A walker steps across generated ground, round a wall two blocks high that
/// it cannot climb, and up a single block that it can.
#[test]
fn a_walker_crosses_generated_ground_and_respects_its_steps() {
    let mut world = World::default();
    let floor = floor(&mut world);
    // A wall across row 3 from column 0 to 5, two high, with a one-block step
    // at column 6 and open ground beyond.
    let mut wall = Vec::new();
    for column in 0..=5 {
        for level in 1..=2 {
            wall.push(json!({ "at": [column, GROUND + level, 3], "block": "stone" }));
        }
    }
    wall.push(json!({ "at": [6, GROUND + 1, 3], "block": "stone" }));
    world
        .get_mut(floor)
        .unwrap()
        .components
        .get_mut(VoxelWorldComponent::TYPE_NAME)
        .unwrap()["edits"] = json!(wall);

    let walker = spawn(
        &mut world,
        "walker",
        "Walker",
        &json!({
            "sindri.grid.occupant": { "grid": "floor", "footprint": [[0, 0]] },
            "sindri.script": { "source": "voxels.decay", "script": "Walker" }
        }),
    );
    world.get_mut(walker).unwrap().transform_3d = Some(Transform3D {
        position: [2.0, 5.0, 1.0],
        ..Transform3D::default()
    });
    let goal = spawn(&mut world, "goal", "Goal", &json!({}));
    world.get_mut(goal).unwrap().transform_3d = Some(Transform3D {
        position: [2.0, 5.0, 6.0],
        ..Transform3D::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "voxels.decay",
        r#"
        script Walker {
            fn update(dt: f32) {
                let floor = World.find("Floor");
                let goal = World.find("Goal");
                if Grid.can_reach(this.entity, floor, goal) {
                    Grid.step_toward(this.entity, floor, goal);
                }
            }
        }
        "#,
    );

    let mut scripts = Scripts::new();
    let sets = tile_sets();
    let input = InputState::default();
    let mut visited = Vec::new();
    for _ in 0..30 {
        let failures = scripts
            .advance(
                &mut world,
                &registry(),
                ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_tile_sets(&sets),
            )
            .failures;
        assert!(failures.is_empty(), "{failures:?}");
        let at = world.get(walker).unwrap().transform_3d.unwrap().position;
        #[allow(clippy::cast_possible_truncation)]
        visited.push((at[0].round() as i32, at[2].round() as i32));
    }
    assert_eq!(visited.last(), Some(&(2, 6)), "it arrives: {visited:?}");
    assert!(
        visited.contains(&(6, 3)),
        "it crosses the wall at its one-block step: {visited:?}"
    );
    assert!(
        !visited
            .iter()
            .any(|(column, row)| *row == 3 && *column <= 5),
        "it never climbs the two-block wall: {visited:?}"
    );
}
