//! A game of Causeway opened from its folder, and the hands to play it.

use glam::Vec3;
use sindri_core::{EntityId, SceneComponent};
use sindri_platform::{InputEvent, Key, MouseButton};
use sindri_runtime::ProjectRun;
use sindri_scene::ModelComponent;

pub const STEP: f32 = 1.0 / 60.0;
pub const SIZE: [f32; 2] = [1280.0, 720.0];
pub const TITLE: f32 = 0.0;
pub const PLAYER: f32 = 2.0;

/// Where cell (0, 0) of the board is in the world; `battle.decay` says the same.
const ORIGIN: f32 = 24.0;

pub struct Game {
    run: ProjectRun,
}

impl Game {
    pub fn open() -> Self {
        let mut run = ProjectRun::open(&sindri_causeway::project_dir(), SIZE)
            .expect("the game opens from its folder");
        run.step(STEP).expect("the first step runs");
        Self { run }
    }

    pub fn step(&mut self) {
        let report = self.run.step(STEP).expect("the step runs");
        let notes = report.notes();
        assert!(notes.is_empty(), "a step went wrong: {notes:?}");
    }

    pub fn play(&mut self, seconds: f32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        for _ in 0..(seconds / STEP).ceil() as usize {
            self.step();
        }
    }

    pub fn board(&self, name: &str) -> f32 {
        self.run.board(name)
    }

    pub fn phase(&self) -> f32 {
        self.board("phase")
    }

    /// Plays on until it is the wardens' turn and nothing is moving.
    pub fn until_players_turn(&mut self) {
        for _ in 0..60 * 30 {
            self.step();
            if self.phase() == PLAYER {
                // A few more frames, so a hop that just finished has landed.
                self.play(0.1);
                return;
            }
        }
        panic!("the wardens' turn never came; the game is in phase {}", self.phase());
    }

    pub fn tap_key(&mut self, key: Key) {
        self.run.key(key, true);
        self.step();
        self.run.key(key, false);
        self.step();
    }

    /// Every piece drawn with this figure's model.
    pub fn pieces(&self, figure: &str) -> Vec<EntityId> {
        let asset = format!("models/{figure}.glb");
        self.run
            .world
            .entities()
            .filter(|(entity, data)| {
                self.run.world.is_active(*entity)
                    && data
                        .components
                        .get(ModelComponent::TYPE_NAME)
                        .and_then(|model| model.get("asset"))
                        .and_then(serde_json::Value::as_str)
                        == Some(asset.as_str())
                    && data.transform_3d.is_some_and(|t| t.scale[1] > 0.1)
            })
            .map(|(entity, _)| entity)
            .collect()
    }

    /// The cell a piece stands on.
    pub fn cell_of(&self, entity: EntityId) -> (i32, i32) {
        let position = self
            .run
            .world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .expect("a piece is placed")
            .position;
        #[allow(clippy::cast_possible_truncation)]
        (
            (position[0] - ORIGIN).floor() as i32,
            (position[2] - ORIGIN).floor() as i32,
        )
    }

    /// The pixel over the middle of a cell's top, as the camera draws it.
    fn pixel_of(&self, cell: (i32, i32), top: f32) -> [f32; 2] {
        let camera = sindri_scene::world_camera_of(
            &self.run.world,
            self.run.scene.components(),
            SIZE[0] / SIZE[1],
        )
        .expect("the camera resolves")
        .expect("the scene has a world camera");
        #[allow(clippy::cast_precision_loss)]
        let point = Vec3::new(
            ORIGIN + cell.0 as f32 + 0.5,
            top,
            ORIGIN + cell.1 as f32 + 0.5,
        );
        let clip = camera.view_projection * point.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        [
            (ndc.x + 1.0) * 0.5 * SIZE[0],
            (1.0 - ndc.y) * 0.5 * SIZE[1],
        ]
    }

    /// The height of a cell's top, read from where a piece standing there
    /// would be -- or, for an empty cell, from the island's own heights.
    fn top_of(&self, cell: (i32, i32)) -> f32 {
        const SALTMARSH: &str = "00111100012332101234432112333211123322110122211100122110\
                                 00011100";
        let index = usize::try_from(cell.1 * 8 + cell.0).expect("a cell on the board");
        let digit = SALTMARSH.as_bytes()[index] - b'0';
        f32::from(digit)
    }

    /// A click on the middle of a cell, pressed and released as a host
    /// delivers one.
    pub fn click_cell(&mut self, cell: (i32, i32)) {
        let [x, y] = self.pixel_of(cell, self.top_of(cell));
        self.run.input.apply(InputEvent::PointerMoved { x, y });
        self.step();
        self.run
            .input
            .apply(InputEvent::ButtonPressed(MouseButton::Left));
        self.step();
        self.run
            .input
            .apply(InputEvent::ButtonReleased(MouseButton::Left));
        self.step();
        self.step();
    }
}
