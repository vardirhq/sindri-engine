//! Arriving lights the beacon, and how it lights is the scene's choreography:
//! a swell that sounds the chime, then a breath that never ends.

use sindri_causeway::{Session, extractor, world};
use sindri_core::{EntityId, World};
use sindri_platform::InputState;

const VIEWPORT: (f32, f32) = (1000.0, 720.0);
const STEP: f32 = 1.0 / 60.0;

fn beacon(world: &World) -> EntityId {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Beacon"))
        .map(|(entity, _)| entity)
        .expect("the scene has a beacon")
}

fn scale(world: &World, entity: EntityId) -> [f32; 3] {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("the beacon has a transform")
        .scale
}

fn playing(world: &World, entity: EntityId) -> Option<String> {
    world.get(entity).expect("the beacon").components["sindri.sequence"]["playing"]
        .as_str()
        .map(str::to_owned)
}

fn play(world: &mut World, session: &mut Session, seconds: f32) {
    let idle = InputState::default();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    for _ in 0..(seconds / STEP).round() as usize {
        session
            .step(world, &idle, VIEWPORT, STEP)
            .expect("a step")
            .log();
    }
}

#[test]
fn arriving_swells_the_beacon_then_it_breathes() {
    let scene = extractor().expect("the schemas register");
    let (mut world, loaded) = world().expect("the world loads");
    let mut session = sindri_causeway::session(scene.components().clone())
        .with_scenes(sindri_causeway::scenes().expect("the scenes load"), loaded)
        .with_tile_sets(sindri_causeway::bind_tile_sets().expect("the tile set binds"));
    let beacon = beacon(&world);
    let rest = scale(&world, beacon);

    play(&mut world, &mut session, 0.5);
    assert_eq!(playing(&world, beacon), None, "nothing lights it before");
    let still = scale(&world, beacon);
    assert!(
        (still[0] - rest[0]).abs() < 1e-6 && (still[1] - rest[1]).abs() < 1e-6,
        "it stays as authored: {still:?}"
    );

    session.set_board("arrived", 1.0);
    play(&mut world, &mut session, 0.5);
    assert_eq!(playing(&world, beacon).as_deref(), Some("arrive"));
    let swelling = scale(&world, beacon);
    assert!(
        swelling[0] > rest[0] * 1.3 && swelling[1] > rest[1] * 1.3,
        "it swells on arriving: {swelling:?} from {rest:?}"
    );

    play(&mut world, &mut session, 1.5);
    assert_eq!(
        playing(&world, beacon).as_deref(),
        Some("glow"),
        "the swell settles into the breath"
    );
    let mut low = f32::INFINITY;
    let mut high = 0.0f32;
    for _ in 0..140 {
        play(&mut world, &mut session, STEP);
        let x = scale(&world, beacon)[0];
        low = low.min(x);
        high = high.max(x);
    }
    assert!(
        low >= rest[0] * 1.2 && high > low * 1.04,
        "it breathes, lit: {low}..{high} from {}",
        rest[0]
    );
    assert!(!session.sequences().is_finished(beacon), "the breath loops");
}
