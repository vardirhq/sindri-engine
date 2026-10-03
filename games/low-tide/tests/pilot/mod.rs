//! A player for the tests: walks the crew and steers the crawler with the
//! keys a person would press, reading only what the screen shows them.

// Shared by more than one test binary, each using only some of it.
#![allow(dead_code)]

pub mod construction;
pub mod salvage;

use std::f32::consts::{PI, TAU};

use low_tide::{Finger, Run};
use sindri_core::EntityId;
use sindri_platform::Key;
use sindri_scene::VoxelGround;

pub const STEP: f32 = 1.0 / 60.0;

/// The crawler's deck, in tiles.
const DECK_COLUMNS: f32 = 11.0;
const DECK_ROWS: f32 = 14.0;

pub struct Pilot {
    pub run: Run,
    pub crew: EntityId,
    pub crawler: EntityId,
    held: [bool; 4],
    /// Playing on a phone: a thumb on the stick instead of the movement keys,
    /// and the buttons instead of E.
    touch: bool,
    thumb_down: bool,
    /// Every press is a new finger to the input system, as on a real screen.
    next_finger: u64,
    steering: u64,
}

/// Where the steering thumb lands, and how far it pulls for a full push,
/// in pixels on a phone held upright.
const THUMB: [f32; 2] = [110.0, 560.0];
const THUMB_REACH: f32 = 130.0;

const KEYS: [Key; 4] = [Key::W, Key::A, Key::S, Key::D];

impl Pilot {
    pub fn new() -> Self {
        let run = Run::open().expect("the project opens");
        let crew = run.entity("crew").expect("the crew");
        let crawler = run.entity("crawler").expect("the crawler");
        Self {
            run,
            crew,
            crawler,
            held: [false; 4],
            touch: false,
            thumb_down: false,
            next_finger: 1,
            steering: 0,
        }
    }

    /// The same voyage on a phone held upright, played by touch.
    pub fn on_a_phone() -> Self {
        let mut pilot = Self::new();
        pilot.run.screen = [390.0, 844.0];
        pilot.touch = true;
        pilot
    }

    /// Taps a screen element with a second finger.
    pub fn press(&mut self, id: &str) {
        let element = self.run.entity(id).expect("the element");
        // Laid out on the step before, as a host lays out what it drew.
        self.step();
        let at = self
            .run
            .on_screen(element)
            .unwrap_or_else(|| panic!("{id} is on screen"));
        let finger = self.new_finger();
        self.run.finger(finger, Finger::Down, at);
        self.step();
        self.run.finger(finger, Finger::Up, at);
        self.step();
    }

    /// Whether this pilot plays by touch.
    pub const fn touch(&self) -> bool {
        self.touch
    }

    /// E, or the Use button.
    pub fn use_it(&mut self) {
        if self.touch {
            self.press(if self.flag("building") {
                "build-confirm-button"
            } else {
                "use-button"
            });
        } else {
            self.tap(Key::E);
        }
    }

    pub fn step(&mut self) {
        let notes = self.run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }

    pub fn wait(&mut self, seconds: f32) {
        self.release();
        for _ in 0..frames(seconds) {
            self.step();
        }
    }

    pub fn board(&self, name: &str) -> f32 {
        self.run.board(name)
    }

    pub fn flag(&self, name: &str) -> bool {
        self.run.board(name) > 0.5
    }

    /// Taps a key for one step.
    pub fn tap(&mut self, key: Key) {
        self.run.key(key, true);
        self.step();
        self.run.key(key, false);
        self.step();
    }

    /// Holds the movement keys that push toward a direction on screen.
    pub fn push(&mut self, screen: [f32; 2]) {
        if self.touch {
            self.thumb(screen);
            return;
        }
        let want = [
            screen[1] > 0.25,
            screen[0] < -0.25,
            screen[1] < -0.25,
            screen[0] > 0.25,
        ];
        for (index, key) in KEYS.iter().enumerate() {
            if want[index] != self.held[index] {
                self.run.key(*key, want[index]);
                self.held[index] = want[index];
            }
        }
    }

    pub fn release(&mut self) {
        self.push([0.0, 0.0]);
    }

    /// Drags the steering thumb toward a direction on screen, up positive,
    /// or lifts it.
    fn thumb(&mut self, screen: [f32; 2]) {
        if length(screen) < 0.01 {
            if self.thumb_down {
                self.run.finger(self.steering, Finger::Up, THUMB);
                self.thumb_down = false;
            }
            return;
        }
        if !self.thumb_down {
            self.steering = self.new_finger();
            self.run.finger(self.steering, Finger::Down, THUMB);
            self.thumb_down = true;
            self.step();
        }
        let way = unit(screen);
        let at = [
            THUMB[0] + way[0] * THUMB_REACH,
            THUMB[1] - way[1] * THUMB_REACH,
        ];
        self.run.finger(self.steering, Finger::Move, at);
    }

    fn new_finger(&mut self) -> u64 {
        self.next_finger += 1;
        self.next_finger
    }

    /// The Basin as gameplay reads it.
    pub fn ground(&self) -> VoxelGround {
        let basin = self.run.entity("basin").expect("the Basin");
        VoxelGround::of_entity(
            &self.run.world,
            &self.run.components,
            basin,
            Some(&self.run.tile_sets),
        )
        .expect("the Basin reads")
        .expect("the Basin is a voxel world")
    }

    /// The block on top under a point in the plane.
    pub fn surface_under(&self, at: [f32; 2]) -> String {
        let ground = self.ground();
        let (column, row) = column_row(at);
        ground
            .surface(column, row)
            .map(|(level, _, _)| ground.block([column, level, row]))
            .unwrap_or_default()
    }

    /// The middle of the nearest column topped with `block` to the crawler,
    /// within sixty units.
    pub fn nearest(&self, block: &str) -> Option<[f32; 2]> {
        let ground = self.ground();
        let start = self.crawler_world();
        let (cx, cz) = column_row(start);
        let mut best: Option<([f32; 2], f32)> = None;
        for z in cz - 60..=cz + 60 {
            for x in cx - 60..=cx + 60 {
                let Some((level, _, _)) = ground.surface(x, z) else {
                    continue;
                };
                if ground.block([x, level, z]) != block {
                    continue;
                }
                #[allow(clippy::cast_precision_loss)]
                let at = [x as f32 + 0.5, -(z as f32) - 0.5];
                let far = distance(at, start);
                if best.is_none_or(|(_, known)| far < known) {
                    best = Some((at, far));
                }
            }
        }
        best.map(|(at, _)| at)
    }

    /// The crew's position in the deck's floor plan, while aboard.
    pub fn on_deck(&self) -> [f32; 2] {
        self.run.local(self.crew)
    }

    pub fn crew_world(&self) -> [f32; 2] {
        self.run.position(self.crew)
    }

    pub fn crawler_world(&self) -> [f32; 2] {
        self.run.position(self.crawler)
    }

    /// A point of the floor plan, in the world, where the crawler is now.
    pub fn deck_to_world(&self, local: [f32; 2]) -> [f32; 2] {
        let heading = self.board("heading");
        let from_middle = turned(
            [local[0] - DECK_COLUMNS / 2.0, local[1] + DECK_ROWS / 2.0],
            heading,
        );
        let middle = self.crawler_world();
        [middle[0] + from_middle[0], middle[1] + from_middle[1]]
    }

    /// Walks the crew aboard through each point of the floor plan in turn.
    pub fn walk_deck(&mut self, points: &[[f32; 2]]) {
        for point in points {
            let mut frames_left = frames(8.0);
            loop {
                let at = self.on_deck();
                let gap = [point[0] - at[0], point[1] - at[1]];
                if length(gap) < 0.12 {
                    break;
                }
                assert!(
                    frames_left > 0,
                    "walking to {point:?} on deck, stuck at {at:?}"
                );
                frames_left -= 1;
                // The keys are screen directions; the deck is turned on screen
                // by however far the crawler is turned past the camera.
                let screen = turned(unit(gap), self.board("heading") - self.board("view_rot"));
                self.push(screen);
                self.step();
                if !self.flag("aboard") {
                    break;
                }
            }
        }
        self.release();
    }

    /// Walks the crew across the salt to a point in the world, until within
    /// `close` of it or back aboard.
    pub fn walk_ashore(&mut self, target: [f32; 2], close: f32) {
        for _ in 0..frames(20.0) {
            let at = self.crew_world();
            let gap = [target[0] - at[0], target[1] - at[1]];
            if length(gap) < close || self.flag("aboard") {
                self.release();
                return;
            }
            let screen = turned(unit(gap), -self.board("view_rot"));
            self.push(screen);
            self.step();
        }
        panic!(
            "never reached {target:?}; the crew is at {:?}",
            self.crew_world()
        );
    }

    /// From where the crew starts to the helm, round the mess table.
    pub fn take_the_helm(&mut self) {
        self.walk_deck(&[[5.5, -4.8]]);
        self.use_it();
        assert!(self.flag("at_helm"), "at the helm");
    }

    /// Holds W or S at the helm until the throttle lever reaches `level`.
    pub fn set_throttle(&mut self, level: f32) {
        for _ in 0..frames(5.0) {
            let now = self.board("throttle");
            let there = if level <= 0.0 {
                now <= 0.0
            } else {
                (now - level).abs() < 0.02
            };
            if there {
                break;
            }
            self.push([0.0, if now < level { 1.0 } else { -1.0 }]);
            self.step();
        }
        self.release();
    }

    /// One step of steering toward a point in the world, from the helm.
    pub fn steer_toward(&mut self, target: [f32; 2]) {
        let at = self.crawler_world();
        let wanted = (-(target[0] - at[0])).atan2(target[1] - at[1]);
        let error = wrapped(wanted - self.board("heading"));
        let turn = if error > 0.04 {
            -1.0
        } else if error < -0.04 {
            1.0
        } else {
            0.0
        };
        let throttle = self.board("throttle");
        self.push([turn, if throttle < 1.0 { 1.0 } else { 0.0 }]);
        self.step();
    }
}

/// The Basin's column and row under a point in the plane.
pub fn column_row(at: [f32; 2]) -> (i32, i32) {
    #[allow(clippy::cast_possible_truncation)]
    let found = (at[0].floor() as i32, (-at[1]).floor() as i32);
    found
}

pub fn frames(seconds: f32) -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = (seconds / STEP).round() as u32;
    count
}

pub fn turned(v: [f32; 2], angle: f32) -> [f32; 2] {
    let (s, c) = angle.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

pub fn length(v: [f32; 2]) -> f32 {
    v[0].hypot(v[1])
}

pub fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    length([a[0] - b[0], a[1] - b[1]])
}

fn unit(v: [f32; 2]) -> [f32; 2] {
    let size = length(v).max(1.0e-6);
    [v[0] / size, v[1] / size]
}

pub fn wrapped(mut angle: f32) -> f32 {
    while angle > PI {
        angle -= TAU;
    }
    while angle < -PI {
        angle += TAU;
    }
    angle
}
