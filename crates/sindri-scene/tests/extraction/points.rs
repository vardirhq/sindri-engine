//! Point lights follow composed transforms, live changes and bounded selection.
use super::support::{document, world_from};
use sindri_core::{EntityId, SceneEntityId, World};
use sindri_scene::SceneExtractor;

fn id(world: &World, source: &str) -> EntityId {
    world
        .entity_for_source_id(&SceneEntityId::new(source).unwrap())
        .unwrap()
}

fn lamp(name: &str, intensity: u8) -> String {
    format!(
        r#"{{"id":"{name}","transform_3d":{{}},"components":{{"sindri.light":{{"kind":"point","intensity":{intensity}}}}}}}"#
    )
}

#[test]
fn moving_turning_scaled_parents_move_points_without_scaling_their_range() {
    let parent = r#"{"id":"parent","transform_3d":{"position":[3,0,0],"rotation":[0,0.70710677,0,0.70710677],"scale":[2,2,2]}}"#;
    let child = r#"{"id":"child","parent":"parent","transform_3d":{"position":[1,0,0]},"components":{"sindri.light":{"kind":"point","range":4}}}"#;
    let mut world = world_from(&document(&format!("{parent},{child}")));
    let extractor = SceneExtractor::new().unwrap();
    let first = extractor.lighting(&world).unwrap();
    let point = first.point_lights[0].unwrap();
    assert!(glam::Vec3::from_array(point.position).abs_diff_eq(glam::vec3(3.0, 0.0, -2.0), 1.0e-5));
    assert!((point.range - 4.0).abs() < 1.0e-6);
    assert!(extractor.sun(&world).unwrap().is_none());
    let parent = id(&world, "parent");
    world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = 7.0;
    assert!(
        (extractor.lighting(&world).unwrap().point_lights[0]
            .unwrap()
            .position[0]
            - 7.0)
            .abs()
            < 1.0e-5
    );
    world.get_mut(parent).unwrap().disabled = true;
    assert!(
        extractor
            .lighting(&world)
            .unwrap()
            .point_lights
            .iter()
            .all(Option::is_none)
    );
    world.get_mut(parent).unwrap().disabled = false;
    assert_eq!(
        extractor.lighting(&world).unwrap().point_lights[0]
            .unwrap()
            .intensity
            .to_bits(),
        1.0_f32.to_bits()
    );
}

#[test]
fn strongest_eight_are_selected_and_invalid_overflow_lights_are_not_hidden() {
    let entities = (1..=10)
        .map(|i| lamp(&format!("lamp-{i}"), i))
        .collect::<Vec<_>>()
        .join(",");
    let mut world = world_from(&document(&entities));
    let strict = SceneExtractor::new().unwrap();
    let lighting = strict.lighting(&world).unwrap();
    for (i, point) in lighting.point_lights.into_iter().enumerate() {
        assert!(
            (point.unwrap().intensity - f32::from(10 - u8::try_from(i).unwrap())).abs() < 1.0e-6
        );
    }
    let lowest = id(&world, "lamp-1");
    world
        .get_mut(lowest)
        .unwrap()
        .components
        .get_mut("sindri.light")
        .unwrap()["range"] = serde_json::json!(0);
    assert!(strict.lighting(&world).is_err());
    let mut tolerant = SceneExtractor::new().unwrap();
    tolerant.tolerate_invalid_components();
    assert!(tolerant.lighting(&world).is_ok());
    assert!(!tolerant.problems().is_empty());
    world
        .get_mut(lowest)
        .unwrap()
        .components
        .get_mut("sindri.light")
        .unwrap()["range"] = serde_json::json!(2);
    assert!(strict.lighting(&world).is_ok());
}
