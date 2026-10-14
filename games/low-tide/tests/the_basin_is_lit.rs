//! The Basin is lit: the sun keeps time with the tide, the crawler's lamps
//! come on at dusk, the treads kick up dust and the engine smokes while it
//! drives, and every lit picture has a normal map that loads.
mod pilot;

use pilot::Pilot;
use sindri_scene::SpriteShaderBindings;

fn component<'a>(pilot: &'a Pilot, id: &str, kind: &str) -> &'a serde_json::Value {
    let entity = pilot.run.entity(id).unwrap_or_else(|| panic!("no {id}"));
    &pilot
        .run
        .world
        .get(entity)
        .expect("it is in the world")
        .components[kind]
}

fn intensity(pilot: &Pilot, id: &str) -> f64 {
    component(pilot, id, "sindri.light2d")["intensity"]
        .as_f64()
        .expect("a number")
}

fn playing(pilot: &Pilot, id: &str) -> bool {
    component(pilot, id, "sindri.particles")["playing"]
        .as_bool()
        .expect("a flag")
}

#[test]
fn the_day_turns_with_the_tide_and_the_lamps_follow_it() {
    let mut pilot = Pilot::new();
    pilot.wait(0.2);
    assert!(
        component(&pilot, "environment", "sindri.environment")["lighting_2d"]["enabled"]
            .as_bool()
            .unwrap()
    );
    assert!(
        (pilot.board("daylight") - 1.0).abs() < 1e-3,
        "low water is day"
    );
    let noon = intensity(&pilot, "sun");
    assert!(noon > 2.0, "a bright sun at low water: {noon}");
    assert!(
        intensity(&pilot, "headlight-port") < 1e-3,
        "no headlights by day"
    );

    // The last minute of low water is the golden hour, and the sun reddens.
    pilot.run.session.blackboard_mut().set("tide_time", 170.0);
    pilot.wait(0.1);
    let golden = pilot.board("daylight");
    assert!(golden > 0.4 && golden < 0.8, "golden hour: {golden}");
    let sun = &component(&pilot, "sun", "sindri.light2d")["color"];
    assert!(sun[2].as_f64().unwrap() < 0.5, "an orange sun: {sun}");

    // High water is the night: the moon is dim and the crawler is lit.
    pilot.run.session.blackboard_mut().set("tide_time", 300.0);
    pilot.wait(0.1);
    assert!(pilot.board("daylight") < 1e-3, "high water is night");
    assert!(intensity(&pilot, "sun") < noon * 0.3);
    assert!(intensity(&pilot, "headlight-starboard") > 1.5);
    assert!(intensity(&pilot, "cabin-light-0") > 0.5);
    assert!(intensity(&pilot, "lantern") > 0.5);

    // The ebb is the dawn, and the next low water is day again.
    pilot.run.session.blackboard_mut().set("tide_time", 405.0);
    pilot.wait(0.1);
    assert!((pilot.board("daylight") - 1.0).abs() < 1e-3);
    assert!(intensity(&pilot, "headlight-port") < 1e-3);
}

#[test]
fn driving_kicks_up_dust_and_smoke_that_settle_when_it_stops() {
    let mut pilot = Pilot::new();
    pilot.wait(0.2);
    assert!(!playing(&pilot, "port-dust") && !playing(&pilot, "exhaust"));
    pilot.take_the_helm();
    pilot.set_throttle(1.0);
    pilot.wait(1.0);
    assert!(
        playing(&pilot, "port-dust"),
        "dry ground under moving treads"
    );
    assert!(playing(&pilot, "starboard-dust"));
    assert!(playing(&pilot, "exhaust"), "an open throttle smokes");
    pilot.set_throttle(0.0);
    pilot.wait(3.0);
    assert!(
        !playing(&pilot, "port-dust"),
        "a parked crawler throws none"
    );
    assert!(!playing(&pilot, "exhaust"));
}

#[test]
fn every_lit_picture_has_its_normal_map_and_the_water_shader_compiles() {
    let assets = low_tide::project().join("assets");
    for name in ["ship", "treads", "crew", "wreck", "dive", "terrain", "pipe"] {
        let picture = std::fs::read(assets.join(format!("textures/{name}.png"))).unwrap();
        let normals = std::fs::read(assets.join(format!("textures/{name}_n.png")))
            .unwrap_or_else(|error| panic!("{name}_n.png: {error}"));
        // Width and height sit at the same place in every PNG header.
        assert_eq!(picture[16..24], normals[16..24], "{name}: laid out alike");
    }
    let source = std::fs::read_to_string(assets.join("shaders/underwater.wgsl")).unwrap();
    SpriteShaderBindings::default()
        .bind("shaders/underwater.wgsl", &source)
        .unwrap_or_else(|error| panic!("{error}"));
}
