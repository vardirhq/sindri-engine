//! Authored normals survive checked scene edits and frame extraction.
use super::support::{VIEWPORT, scene, world_from};
use sindri_core::{CommandBuffer, CommandHistory, SceneEntityId, WorldCommand};
use sindri_render::FrameCommand;
use sindri_scene::{CameraView, SceneExtractor, TextureBindings};

#[test]
fn surface_normals_extract_and_restore_through_undo() {
    let mut payload = serde_json::json!({"primitive":"surface", "texture":"t", "surface": {
        "vertices":[[-1,-1,0],[1,-1,0],[0,1,0]], "uvs":[[0,0],[1,0],[0.5,1]], "indices":[0,1,2]
    }});
    let mut world = world_from(&scene(&format!(
        r#", {{"id":"mesh", "components":{{"sindri.mesh":{payload}}}}}"#
    )));
    let extractor = SceneExtractor::new().unwrap();
    let textures = TextureBindings::new();
    let normal = |world: &sindri_core::World| {
        let frame = extractor
            .extract(world, VIEWPORT, CameraView::default(), &textures)
            .unwrap();
        let FrameCommand::TexturedMesh { vertices, .. } = &frame.passes()[0].command else {
            panic!("surface expected");
        };
        vertices
            .iter()
            .map(|v| v.normal.map(f32::to_bits))
            .collect::<Vec<_>>()
    };
    let flat = normal(&world);
    assert!(flat.iter().all(|n| *n == [0.0_f32.to_bits(); 3]));
    payload["surface"]["normals"] = serde_json::json!([[0, 0, 1], [1, 0, 1], [0, 1, 1]]);
    let entity = world
        .entity_for_source_id(&SceneEntityId::new("mesh").unwrap())
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: "sindri.mesh".into(),
        payload,
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Surface Normals"), &mut world)
        .unwrap();
    let authored = normal(&world);
    assert_eq!(
        authored[1],
        [1.0_f32.to_bits(), 0.0_f32.to_bits(), 1.0_f32.to_bits()]
    );
    history.undo(&mut world).unwrap();
    assert_eq!(normal(&world), flat);
    history.redo(&mut world).unwrap();
    assert_eq!(normal(&world), authored);
}
