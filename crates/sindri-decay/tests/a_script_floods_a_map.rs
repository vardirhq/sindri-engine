//! A flood is reversible presentation, and scripts read the rendered threshold.
use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::VoxelWorldComponent;

fn run(source: &str, view: &str) -> (World, Scripts, sindri_decay::ScriptReport) {
    let mut components = ComponentSchemaRegistry::default();
    components.register::<ScriptComponent>("Script").unwrap();
    components.register::<VoxelWorldComponent>("World").unwrap();
    let mut world = World::default();
    world.spawn(EntityData {
        name: Some("Map".into()),
        components: [
            (
                "sindri.voxel_world".into(),
                json!({"view":view,
                "generator":{"kind":"layered_terrain", "base_height":4,"height_variation":0},
                "edits":[{"at":[1,8,0],"block":3}]}),
            ),
            (
                "sindri.script".into(),
                json!({"source":"flood.decay","script":"Flood"}),
            ),
        ]
        .into(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("flood.decay", source);
    let mut scripts = Scripts::new();
    let report = scripts.advance(
        &mut world,
        &components,
        ScriptFrame::new(&sources, &InputState::default(), 0.1),
    );
    (world, scripts, report)
}

#[test]
fn scripts_share_the_maps_threshold_and_clearing_preserves_edits() {
    let (world, scripts, report) = run(
        r#"
        state Game { var boundary: bool = false; var below: bool = false;
            var high: bool = true; var cleared: bool = true; var height: f32 = 0.0; }
        script Flood { fn start() {
            Grid.set_flood(this.entity, 5.0, "2");
            Game.boundary = Grid.flooded(this.entity, 0.0, 0.0);
            Grid.set_flood(this.entity, 6.0, "2");
            Game.below = Grid.flooded(this.entity, 0.0, 0.0);
            Game.high = Grid.flooded(this.entity, 1.0, 0.0);
            Game.height = Grid.height(this.entity, 0.0, 0.0) ?? -1.0;
            Grid.set_flood(this.entity, 6.0, "");
            Game.cleared = Grid.flooded(this.entity, 0.0, 0.0);
        } }
    "#,
        "map",
    );
    assert!(report.failures.is_empty(), "{report:?}");
    let board = scripts.blackboard();
    assert!(board.get("boundary", 1.0).abs() < 0.01);
    assert!((board.get("below", 0.0) - 1.0).abs() < 0.01);
    assert!(board.get("high", 1.0).abs() < 0.01);
    assert!(board.get("cleared", 1.0).abs() < 0.01);
    assert!((board.get("height", 0.0) - 5.0).abs() < 0.01);
    let payload = world
        .entities()
        .next()
        .unwrap()
        .1
        .components
        .get("sindri.voxel_world")
        .unwrap();
    assert!(payload["map_flood"].is_null());
    assert_eq!(payload["edits"], json!([{"at":[1,8,0],"block":3}]));
}

#[test]
fn a_bad_water_block_fails_at_the_call_without_changing_the_world() {
    let (world, _, report) = run(
        r#"script Flood { fn start() {
        Grid.set_flood(this.entity, 6.0, "missing");
    } }"#,
        "map",
    );
    assert_eq!(report.failures.len(), 1);
    let payload = world
        .entities()
        .next()
        .unwrap()
        .1
        .components
        .get("sindri.voxel_world")
        .unwrap();
    assert!(payload.get("map_flood").is_none());
}

#[test]
fn block_views_refuse_the_map_only_overlay() {
    let (_, _, report) = run(
        r#"script Flood { fn start() {
        Grid.set_flood(this.entity, 6.0, "2");
    } }"#,
        "blocks",
    );
    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].to_string().contains("view: map"));
}
