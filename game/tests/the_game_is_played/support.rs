//! Playing the game the way a person does: a session, a camera, clicks and
//! taps, and the world they change.

use glam::{Mat4, Vec3};
use sindri_causeway::{Session, extractor, world};
use sindri_core::World;
use sindri_platform::{InputEvent, InputState, MouseButton};
use sindri_scene::{SceneExtractor, UiTextComponent, VoxelGround, VoxelWorldComponent};

/// The level water fills to in Causeway's world.
pub(crate) const SEA: i32 = 0;

pub(crate) const VIEWPORT: (f32, f32) = (1000.0, 720.0);
pub(crate) const STEP: f32 = 1.0 / 60.0;

pub(crate) fn session() -> (World, SceneExtractor, Session) {
    let scene = extractor().expect("the schemas register");
    let (world, loaded) = world().expect("the world loads");
    let session = sindri_causeway::session(scene.components().clone())
        .with_scenes(sindri_causeway::scenes().expect("the scenes load"), loaded)
        .with_tile_sets(sindri_causeway::bind_tile_sets().expect("the tile set binds"));
    (world, scene, session)
}

pub(crate) fn view_projection(world: &World, scene: &SceneExtractor) -> Mat4 {
    sindri_scene::world_camera_of(world, scene.components(), VIEWPORT.0 / VIEWPORT.1)
        .expect("the camera resolves")
        .expect("the scene has a world camera")
        .view_projection
}

pub(crate) fn project(point: Vec3, view_projection: Mat4) -> [f32; 2] {
    let clip = view_projection * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    [
        (ndc.x + 1.0) * 0.5 * VIEWPORT.0,
        (1.0 - ndc.y) * 0.5 * VIEWPORT.1,
    ]
}

pub(crate) fn click(world: &mut World, session: &mut Session, at: [f32; 2], button: MouseButton) {
    let mut held = InputState::default();
    held.apply(InputEvent::PointerMoved { x: at[0], y: at[1] });
    held.apply(InputEvent::ButtonPressed(button));
    session
        .step(world, &held, VIEWPORT, STEP)
        .expect("the press steps")
        .log();
    // A frame boundary, exactly as a host puts one there. Without it the press
    // edge is still set on the next step and one click lays two blocks.
    held.begin_frame(std::time::Duration::from_secs_f32(STEP));
    held.apply(InputEvent::ButtonReleased(button));
    session
        .step(world, &held, VIEWPORT, STEP)
        .expect("the release steps")
        .log();
    // One frame with nothing down, which is what a host does between two
    // clicks. Without it the finished press is still in the set when the next
    // one starts, and the recogniser -- which keeps what it has decided about
    // a press for as long as the press exists -- carries the first click's
    // verdict onto the second.
    held.begin_frame(std::time::Duration::from_secs_f32(STEP));
    session
        .step(world, &held, VIEWPORT, STEP)
        .expect("the empty frame steps")
        .log();
}

/// Holding still, which is how a block is taken back.
///
/// A finger has no second button, so removing is a press that stays put and
/// stays down past the long-press limit. The press is left down for a good
/// while rather than one step, because that duration is the whole gesture.
pub(crate) fn hold(world: &mut World, session: &mut Session, at: [f32; 2]) {
    let mut held = InputState::default();
    held.apply(InputEvent::PointerMoved { x: at[0], y: at[1] });
    held.apply(InputEvent::ButtonPressed(MouseButton::Left));
    session
        .step(world, &held, VIEWPORT, STEP)
        .expect("the press steps")
        .log();
    // Past the long-press limit without moving. Reported once, while the
    // finger is still down, so one hold takes one block.
    held.begin_frame(std::time::Duration::from_millis(600));
    session
        .step(world, &held, VIEWPORT, STEP)
        .expect("the hold steps")
        .log();
    held.begin_frame(std::time::Duration::from_secs_f32(STEP));
    held.apply(InputEvent::ButtonReleased(MouseButton::Left));
    session
        .step(world, &held, VIEWPORT, STEP)
        .expect("the release steps")
        .log();
}

pub(crate) fn tap_touch(
    world: &mut World,
    scene: &SceneExtractor,
    session: &mut Session,
    at: [f32; 2],
) -> [i32; 3] {
    let mut input = InputState::default();
    input.apply(InputEvent::TouchStarted {
        id: 7,
        x: at[0],
        y: at[1],
    });
    session
        .step(world, &input, VIEWPORT, STEP)
        .expect("the touch press steps")
        .log();

    input.begin_frame(std::time::Duration::from_secs_f32(STEP));
    input.apply(InputEvent::TouchEnded { id: 7 });
    // Play's follow camera can advance between press and release. Capture the
    // release-frame Aim the script will consume rather than assuming touch-down
    // and touch-up project onto the same voxel.
    let aim = aim(world, scene, [at[0] / VIEWPORT.0, at[1] / VIEWPORT.1])
        .expect("the released touch still points at the world");
    let touched = [aim.cell.x, aim.cell.y, aim.cell.z];
    session
        .step(world, &input, VIEWPORT, STEP)
        .expect("the touch release steps")
        .log();

    input.begin_frame(std::time::Duration::from_secs_f32(STEP));
    session
        .step(world, &input, VIEWPORT, STEP)
        .expect("the empty touch frame steps")
        .log();
    touched
}

pub(crate) fn entity_position(world: &World, name: &str) -> [f32; 3] {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some(name))
        .and_then(|(_, data)| data.transform_3d)
        .expect("named entity has a transform")
        .position
}

pub(crate) fn planar_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dx = left[0] - right[0];
    let dz = left[2] - right[2];
    (dx * dx + dz * dz).sqrt()
}

pub(crate) fn ui_text(world: &World, scene: &SceneExtractor, name: &str) -> UiTextComponent {
    let entity = world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some(name))
        .map(|(entity, _)| entity)
        .expect("named UI entity exists");
    scene
        .components()
        .get::<UiTextComponent>(world, entity)
        .expect("the UI text schema reads")
        .expect("the named UI text is active")
}

pub(crate) fn settle(world: &mut World, session: &mut Session, steps: usize) {
    let idle = InputState::default();
    for _ in 0..steps {
        session
            .step(world, &idle, VIEWPORT, STEP)
            .expect("an idle step")
            .log();
    }
}

/// The world's ground as it stands now, generated and edited.
pub(crate) fn ground(world: &World, scene: &SceneExtractor) -> VoxelGround {
    let (_, component) = scene
        .components()
        .query::<VoxelWorldComponent>(world)
        .expect("the voxel world schema reads")
        .into_iter()
        .next()
        .expect("the world has a voxel world");
    let tile_sets = sindri_causeway::bind_tile_sets().expect("the tile set binds");
    VoxelGround::of(&component, Some(&tile_sets)).expect("the world reads")
}

/// The block in a grid cell -- a column, a row and a level -- if any.
pub(crate) fn block_at(world: &World, scene: &SceneExtractor, cell: [i32; 3]) -> Option<String> {
    let block = ground(world, scene).block([cell[0], cell[2], cell[1]]);
    (!block.is_empty()).then_some(block)
}

/// Where a click on the picture lands, as the session aims it.
pub(crate) fn aim(
    world: &World,
    scene: &SceneExtractor,
    point: [f32; 2],
) -> Option<sindri_scene::voxel::VolumeAim> {
    let tile_sets = sindri_causeway::bind_tile_sets().expect("the tile set binds");
    sindri_scene::voxel::aim_at_with(
        world,
        scene.components(),
        Some(&tile_sets),
        view_projection(world, scene),
        point,
    )
}

/// A block the camera can actually see, and where on the picture it is.
///
/// Asked of the picker rather than worked out from the world, and the
/// difference matters: a column can be perfectly good ground and still be
/// behind a hill from here, and a click aimed at one lands on the hill. What a
/// test wants is somewhere a *player* could click, which is exactly what the
/// pointer already answers.
pub(crate) fn in_view(world: &World, scene: &SceneExtractor) -> ([i32; 3], [f32; 2]) {
    for (across, down) in [(0.5, 0.55), (0.42, 0.6), (0.58, 0.5), (0.5, 0.45)] {
        let at = [across * VIEWPORT.0, down * VIEWPORT.1];
        if let Some(aim) = aim(world, scene, [across, down])
            && aim.face == sindri_core::TileFace::Top
        {
            return ([aim.cell.x, aim.cell.y, aim.cell.z], at);
        }
    }
    panic!("nothing in the middle of the picture to point at");
}

/// A visible top face outside one logical grid cell.
///
/// Play starts with Target under the Wanderer and the camera centred there,
/// so the ordinary centre-of-view helper can legitimately return Target's
/// current cell. A movement regression must choose somewhere else or a
/// correctly handled tap can look like no movement at all.
pub(crate) fn in_view_away_from(
    world: &World,
    scene: &SceneExtractor,
    excluded: [i32; 2],
) -> ([i32; 3], [f32; 2]) {
    for (across, down) in [
        (0.32, 0.58),
        (0.68, 0.42),
        (0.36, 0.42),
        (0.64, 0.58),
        (0.5, 0.68),
        (0.5, 0.32),
    ] {
        let at = [across * VIEWPORT.0, down * VIEWPORT.1];
        if let Some(aim) = aim(world, scene, [across, down])
            && aim.face == sindri_core::TileFace::Top
            && (aim.cell.x - excluded[0]).abs() + (aim.cell.y - excluded[1]).abs() >= 4
        {
            return ([aim.cell.x, aim.cell.y, aim.cell.z], at);
        }
    }
    panic!("nothing outside {excluded:?} is visible to tap");
}
