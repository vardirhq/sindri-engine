//! Stage 2 of `docs/drift-plan.md`: the tether. A line of physics links on
//! 3D distance joints from the aft handhold to the diver: it stops the diver
//! at its length without a rebound, catches on what it is drawn across, reels
//! the diver home, and can be unclipped and clipped back on.

use drift::Run;
use sindri_platform::Key;

const LINE: f32 = 10.0;
const LINKS: usize = 16;

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}

/// The line's points from the handhold through every link to the diver.
fn line(game: &Run) -> Vec<[f32; 3]> {
    let mut points = vec![game.position("start-hold")];
    points.extend((0..LINKS).map(|i| game.position(&format!("tether-link-{i}"))));
    points.push(game.position("diver"));
    points
}

fn along(points: &[[f32; 3]]) -> f32 {
    points.windows(2).map(|pair| dist(pair[0], pair[1])).sum()
}

fn from_hold(game: &Run) -> f32 {
    dist(game.position("start-hold"), game.position("diver"))
}

#[test]
fn the_line_stops_the_diver_at_its_length_and_reels_them_home() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.3);
    assert!(game.has_said("Drift: tether clipped on"), "{:?}", game.said);
    game.tap(Key::Space);
    let mut farthest = 0.0_f32;
    for _ in 0..360 {
        game.step();
        farthest = farthest.max(from_hold(&game));
    }
    assert!(game.has_said("Drift: tether taut"), "{:?}", game.said);
    assert!(
        farthest < LINE + 0.6,
        "the line holds the diver: farthest {farthest}"
    );
    assert!(farthest > LINE - 1.5, "the diver got out to the line's end");
    // Brought up short, the diver hangs at the end rather than bouncing back.
    let hanging = from_hold(&game);
    game.wait(1.0);
    assert!(
        (from_hold(&game) - hanging).abs() < 0.3,
        "a tug, not a rebound: {hanging} then {}",
        from_hold(&game)
    );
    let said_taut = game
        .said
        .iter()
        .filter(|line| *line == "Drift: tether taut");
    assert_eq!(
        said_taut.count(),
        1,
        "one tug, not a flutter: {:?}",
        game.said
    );

    game.press(Key::R);
    assert!(game.until("Drift: reeling in", 0.1));
    game.wait(8.0);
    game.release(Key::R);
    assert!(
        from_hold(&game) < 1.5,
        "reeled home: {} from the handhold",
        from_hold(&game)
    );
}

#[test]
fn the_line_catches_on_the_stanchion_and_draws_the_diver_round_it() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.3);
    // Down the hold past the stanchion, across behind it, and back towards
    // the start on its far side, braking at each turn.
    game.tap(Key::Space);
    game.wait(1.8);
    for (key, burn, coast) in [(Key::D, 0.6, 1.0), (Key::S, 0.6, 2.0)] {
        game.press(Key::X);
        game.wait(1.5);
        game.release(Key::X);
        game.press(key);
        game.wait(burn);
        game.release(key);
        game.wait(coast);
    }
    game.press(Key::X);
    game.wait(1.0);
    game.release(Key::X);
    let behind = game.position("diver");
    assert!(behind[2] > 1.5, "the diver is across the hold: {behind:?}");

    game.press(Key::R);
    game.wait(4.0);
    let points = line(&game);
    let stanchion = game.position("stanchion");
    let touching = points[1..=LINKS]
        .iter()
        .map(|p| ((p[0] - stanchion[0]).powi(2) + (p[2] - stanchion[2]).powi(2)).sqrt())
        .fold(f32::MAX, f32::min);
    // Half the stanchion's width plus a link's radius, and a little more.
    assert!(
        touching < 0.3,
        "a link lies against the stanchion: {touching}"
    );
    let diver = game.position("diver");
    let straight = dist(points[0], diver);
    assert!(
        along(&points) > straight + 0.4,
        "the line bends round the stanchion: {} along, {straight} straight",
        along(&points)
    );
    assert!(
        diver[0] > behind[0] + 0.2,
        "reeling draws the diver towards the stanchion, not the handhold: {behind:?} {diver:?}"
    );
}

#[test]
fn unclipped_the_diver_is_free_and_clips_back_on_only_at_the_loose_end() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.3);
    game.tap(Key::U);
    assert!(game.has_said("Drift: unclipped"), "{:?}", game.said);
    // The loose end is still in reach: clip straight back on.
    game.tap(Key::U);
    let clipped = game
        .said
        .iter()
        .filter(|line| *line == "Drift: tether clipped on")
        .count();
    assert_eq!(clipped, 2, "{:?}", game.said);

    game.tap(Key::U);
    game.tap(Key::Space);
    game.wait(5.0);
    assert!(
        from_hold(&game) > LINE + 1.0,
        "nothing holds an unclipped diver: {}",
        from_hold(&game)
    );
    // Far from the loose end, U finds nothing to clip on to.
    let said = game.said.len();
    game.tap(Key::U);
    game.step();
    assert!(
        !game.said[said..]
            .iter()
            .any(|line| line == "Drift: tether clipped on"),
        "{:?}",
        game.said
    );
    // And reeling does nothing to a diver who is not on the line.
    let before = from_hold(&game);
    game.press(Key::R);
    game.wait(1.0);
    game.release(Key::R);
    assert!(!game.has_said("Drift: reeling in"));
    assert!(
        from_hold(&game) >= before - 0.6,
        "{before} {}",
        from_hold(&game)
    );
}
