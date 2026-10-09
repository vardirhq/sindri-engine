//! The browser smoke's own sequence (`scripts/browser/physics-demo.mjs`),
//! played natively at the uneven paces a browser delivers frames at.
//!
//! The smoke waits in wall-clock time, so how much of the room has moved
//! between two of its keys depends on how fast the page draws. The domino
//! run used to end with its last domino's tip only just reaching the red
//! button's edge, and about one pacing in six left it leaning short of the
//! cap -- the CI failure "never said red button". Each run here is
//! deterministic; together they cover the paces that found it.

use sindri_platform::{InputEvent, Key};

use crate::support::{Playground, times};

/// How many fixed steps a wait of so many milliseconds is, at a pace that
/// wobbles around `scale` of real time the way a busy page's frames do.
struct Pace {
    seed: u64,
    scale: f32,
}

impl Pace {
    fn steps(&mut self, ms: f32) -> usize {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_precision_loss)]
        let wobble = 0.5 + (self.seed >> 33) as f32 / (1_u64 << 31) as f32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (ms / 1000.0 * 60.0 * self.scale * wobble).round() as usize;
        steps.max(1)
    }
}

struct Smoke {
    playground: Playground,
    pace: Pace,
}

impl Smoke {
    fn wait(&mut self, ms: f32) {
        let steps = self.pace.steps(ms);
        self.playground.play(steps);
    }

    /// Waits, a quarter second at a time, for the playground to say `words`
    /// once more than `before`.
    fn wait_for(&mut self, words: &str, seconds: f32, before: usize) -> bool {
        let mut waited = 0.0;
        while waited < seconds && times(words) <= before {
            self.wait(250.0);
            waited += 0.25;
        }
        times(words) > before
    }

    fn press(&mut self, key: Key, words: &str) {
        let before = times(words);
        self.playground.key(key);
        assert!(self.wait_for(words, 10.0, before), "never said {words:?}");
    }

    fn tap_centre(&mut self) {
        let [width, height] = self.playground.size;
        self.playground.press_at([width / 2.0, height * 0.55]);
        self.playground.step();
        self.playground.release();
        self.wait(300.0);
    }
}

fn play_the_smoke(width: f32, height: f32, pace: Pace) -> bool {
    let mut smoke = Smoke {
        playground: Playground::open(width, height),
        pace,
    };
    smoke.tap_centre();
    smoke.playground.key(Key::O);
    smoke.wait(900.0);
    for gravity in ["MOON", "ZERO-G", "UPSIDE DOWN", "SIDEWAYS", "EARTH"] {
        smoke.press(Key::V, &format!("gravity {gravity}"));
    }
    smoke.playground.key(Key::B);
    smoke.tap_centre();
    smoke.wait(500.0);
    smoke.press(Key::I, "debug true");
    smoke.playground.key(Key::X);
    smoke.wait(1000.0);
    smoke.press(Key::E, "toy WRECKING BALL");
    smoke.press(Key::R, "reset WRECKING BALL");
    smoke.press(Key::Q, "toy THE WHOLE ROOM");
    smoke.press(Key::Q, "toy DOMINO RUN");
    smoke.press(Key::Q, "toy TEST TRACK");
    smoke.playground.input.apply(InputEvent::KeyPressed(Key::D));
    smoke.wait(700.0);
    smoke
        .playground
        .input
        .apply(InputEvent::KeyReleased(Key::D));
    smoke.press(Key::F, "robot kick");
    smoke.press(Key::E, "toy DOMINO RUN");
    smoke.press(Key::R, "reset DOMINO RUN");
    smoke.wait(1200.0);
    let presses = times("red button");
    smoke.press(Key::Digit1, "domino pushed");
    smoke.wait_for("red button", 45.0, presses)
}

#[test]
fn the_domino_run_reaches_the_button_however_a_phone_paces_the_smoke() {
    let missed: Vec<u64> = (0..2)
        .filter(|&seed| {
            !play_the_smoke(
                390.0,
                844.0,
                Pace {
                    seed: seed * 7919 + 1,
                    scale: 0.5,
                },
            )
        })
        .collect();
    assert!(
        missed.is_empty(),
        "the run stopped short for seeds {missed:?}"
    );
}
