//! Gathering and building in the wilds (`docs/lamplighter-plan.md`, stage 3):
//! deadwood, flint and resin lie on the ground and go into Wick's pack when he
//! walks over them; with enough in it he builds a campfire, whose light is a
//! safe pool, and which burns down.

use lamplighter::Run;
use sindri_core::EntityId;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn tap(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 2);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 2);
}

fn place(run: &mut Run, entity: EntityId, at: [f32; 2]) {
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

fn at(run: &Run, entity: EntityId) -> [f32; 2] {
    let place = run.world.world_transform(entity).expect("a place").position;
    [place[0], place[1]]
}

/// Every entity running `script`, with where it is.
fn running(run: &Run, script: &str, kind: Option<&str>) -> Vec<(EntityId, [f32; 2])> {
    run.world
        .entities()
        .filter(|(_, data)| {
            data.components.get("sindri.script").is_some_and(|it| {
                it["script"] == script && kind.is_none_or(|kind| it["properties"]["kind"] == kind)
            })
        })
        .map(|(entity, _)| (entity, at(run, entity)))
        .collect()
}

/// Wick walks over deadwood lying near the start, and it goes into his pack.
#[test]
fn wick_gathers_what_he_walks_over() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 5);
    for kind in ["wood", "stone", "resin"] {
        assert!(
            !running(&run, "Pickup", Some(kind)).is_empty(),
            "{kind} lies about the start"
        );
    }
    let (wood, spot) = running(&run, "Pickup", Some("wood"))[0];
    let wick = run.entity("wick").expect("Wick");
    place(&mut run, wick, spot);
    step(&mut run, 3);
    assert!(
        (run.board("wood") - 1.0).abs() < 1e-3,
        "one wood in the pack"
    );
    assert!(run.world.get(wood).is_none(), "taken from the ground");
}

/// With enough in the pack, F puts a ghost of a campfire a step ahead of Wick
/// and F again sets it down there: the cost is taken, its light kindles, and in
/// time it burns down.
#[test]
fn wick_builds_a_campfire_and_it_burns_down() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    assert!(running(&run, "Fire", None).is_empty(), "no fire yet");
    run.input.apply(InputEvent::KeyPressed(Key::F));
    step(&mut run, 2);
    run.input.apply(InputEvent::KeyReleased(Key::F));
    step(&mut run, 2);
    assert!(
        running(&run, "Fire", None).is_empty(),
        "nothing to build it with"
    );

    run.set_board("wood", 5.0);
    run.set_board("stone", 2.0);
    tap(&mut run, Key::F);
    assert!(
        running(&run, "Fire", None).is_empty(),
        "the first press only chooses where"
    );
    assert!(
        (run.board("wood") - 5.0).abs() < 1e-3,
        "and costs nothing yet"
    );
    tap(&mut run, Key::F);
    step(&mut run, 30);
    let fires = running(&run, "Fire", None);
    assert_eq!(fires.len(), 1, "a campfire is built");
    assert!((run.board("wood") - 1.0).abs() < 1e-3, "its wood is spent");
    assert!(run.board("stone").abs() < 1e-3, "and its stone");
    let (fire, spot) = fires[0];
    let wick = at(&run, run.entity("wick").expect("Wick"));
    let apart = (spot[0] - wick[0]).hypot(spot[1] - wick[1]);
    assert!(apart < 2.0, "a step from Wick: {apart}");
    let glow = |run: &Run| {
        run.world.get(fire).expect("the fire").components["sindri.light2d"]["intensity"]
            .as_f64()
            .expect("a number")
    };
    assert!(glow(&run) < 1.0, "it is still catching: {}", glow(&run));
    step(&mut run, 60 * 3);
    assert!(glow(&run) > 1.0, "and then it burns");
    step(&mut run, 60 * 92);
    assert!(glow(&run) < 0.01, "and has burnt down");
}

/// A lamp Wick builds burns for good, and a Seeker cannot find him by it.
#[test]
fn a_lamp_wick_builds_hides_him_from_a_seeker() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    run.set_board("wood", 6.0);
    run.set_board("stone", 4.0);
    run.set_board("resin", 2.0);
    tap(&mut run, Key::G);
    tap(&mut run, Key::G);
    step(&mut run, 10);
    assert_eq!(running(&run, "Fire", None).len(), 1, "a lamp is built");
    assert!(run.board("resin").abs() < 1e-3, "its resin is spent");
    // Once it has caught.
    step(&mut run, 60 * 3);
    let wick = at(&run, run.entity("wick").expect("Wick"));
    let seeker = run.entity("seeker-1").expect("the Seeker");
    place(&mut run, seeker, wick);
    step(&mut run, 60 * 3);
    assert!(run.board("found") < 0.05, "hidden by his own light");
}

/// On a phone everything is tapped: the satchel opens the pack, a press on its
/// campfire plate is the plate's (a host with a stick asks `takes_press` before
/// taking the finger) while open ground is not, and Place sets the fire down.
#[test]
fn a_finger_opens_the_pack_and_builds_a_campfire() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    run.set_board("wood", 4.0);
    run.set_board("stone", 2.0);
    step(&mut run, 2);
    finger(&mut run, "hud-satchel");
    let card = run
        .on_screen(run.entity("craft-campfire").expect("the plate"))
        .expect("on screen once the pack is open");
    assert!(run.takes_press(card), "the plate takes a press at {card:?}");
    let [width, height] = run.size;
    assert!(
        !run.takes_press([width / 2.0, height / 2.0 + 120.0]),
        "open ground does not"
    );
    finger(&mut run, "craft-campfire");
    assert!(
        running(&run, "Fire", None).is_empty(),
        "not until it is placed"
    );
    finger(&mut run, "place-confirm");
    step(&mut run, 10);
    assert_eq!(running(&run, "Fire", None).len(), 1, "a campfire is built");
    assert!(run.board("wood").abs() < 1e-3, "and paid for");
}

fn finger(run: &mut Run, id: &str) {
    run.present().expect("the screen lays out");
    let at = run
        .on_screen(run.entity(id).expect("the element"))
        .unwrap_or_else(|| panic!("{id} is on screen"));
    run.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: at[0],
        y: at[1],
    });
    step(run, 1);
    run.input.apply(InputEvent::TouchEnded { id: 1 });
    step(run, 4);
}

/// By a fire he has set down, the screen lets go of him: once it has caught
/// and he has stood by it a moment, the lantern and the satchel fade, and they
/// come back when he walks away.
#[test]
fn by_a_fire_the_screen_lets_go() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    run.set_board("wood", 4.0);
    run.set_board("stone", 2.0);
    tap(&mut run, Key::F);
    tap(&mut run, Key::F);
    step(&mut run, 20);
    assert!(run.board("resting") < 0.5, "not while it is still catching");
    step(&mut run, 60 * 5);
    assert!(
        run.board("resting") > 0.5,
        "by a fire that has caught, he rests"
    );
    let wick = run.entity("wick").expect("Wick");
    let spot = at(&run, wick);
    place(&mut run, wick, [spot[0] + 30.0, spot[1]]);
    step(&mut run, 3);
    assert!(
        run.board("resting") < 0.5,
        "and walking off, the screen is back"
    );
}

/// The fill a screen element is drawn with this frame: styled, animations
/// and all, as a host draws it.
fn drawn_fill(run: &mut Run, id: &str) -> serde_json::Value {
    let entity = run.entity(id).expect("the element");
    run.draw_with(|_, world, _| {
        let fill = world.get(entity).expect("drawn").components["sindri.ui.shape"]["fill"].clone();
        Ok((fill, sindri_scene::UiTextSizes::new()))
    })
    .expect("it draws")
}

/// A row of the pack shines for a moment when it gains, and goes back.
#[test]
fn the_pack_row_shines_when_wick_gathers() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    let resting = drawn_fill(&mut run, "pack-wood-row");
    run.set_board("wood", 1.0);
    step(&mut run, 2);
    drawn_fill(&mut run, "pack-wood-row");
    step(&mut run, 6);
    let shining = drawn_fill(&mut run, "pack-wood-row");
    assert_ne!(shining, resting, "it shines: {shining} against {resting}");
    for _ in 0..60 {
        step(&mut run, 1);
        drawn_fill(&mut run, "pack-wood-row");
    }
    assert_eq!(
        drawn_fill(&mut run, "pack-wood-row"),
        resting,
        "and goes back"
    );
}
