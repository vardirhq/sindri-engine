//! The native editor shell.
//!
//! `EditorApp` is the whole of the editor's state; everything else here is one
//! region of the window or one thing the user can do to that state. The
//! submodules hold the work, this file holds what they all share: the state
//! itself, the constants they agree on, and how the window is opened.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;
use glam::Vec2 as GlamVec2;
use sindri_core::{CommandHistory, EngineLifecycle, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::ScriptComponent;
use sindri_scene::{
    AudioSourceComponent, CameraComponent, GridNavigationComponent, GridOccupantComponent,
    SceneExtractor, ScenePhysics2d, ScreenUi, SpriteAnimations, SpriteComponent, UiImageComponent,
    UiTextComponent,
};

pub use window::run;

use crate::audition::Audition;
use crate::preview::TextPreview;
use crate::profile::ProfileEditor;
use crate::selection::Selection;
use crate::typeface::Typeface;
use crate::{
    animation::AnimationTool,
    console::Console,
    gizmo::{GizmoDrag, GizmoMode, GizmoSpace},
    input::EditorInput,
    preferences::Preferences,
    project::ProjectTree,
    scene_file::SceneFile,
    scripts::SceneScripts,
    slicer::Slicer,
    textures::SceneTextures,
    tile_volume::TileVolumeTool,
    tilemap::TilemapTool,
    weave_styles::ProjectStyles,
};

mod animated;
mod assistant_view;
mod audio_view;
mod block_pointer;
mod block_set_view;
mod camera;
mod chrome;
mod collider_gizmo;
mod console_view;
mod device;
mod dock_strip;
mod editing;
mod external_editor;
mod frame;
mod gizmo_space;
mod hierarchy;
mod history_view;
mod inspector_panel;
mod instances;
mod light_gizmo;
mod occlusion_view;
mod overlay;
mod palette_view;
mod pointer;
mod prefab_authoring;
pub(crate) mod prefab_drop;
mod prefab_panel;
mod prefab_pointer;
mod prefab_writes;
mod presentation;
mod preview_view;
mod profile_view;
mod profiler_view;
mod project_open;
mod project_panel;
mod projection;
mod repair_view;
mod runtime;
mod scene_board_view;
mod scene_io;
mod scene_lighting;
mod scene_new;
mod shortcuts;
mod slicer_view;
mod startup;
mod thumbnails;
mod timeline_view;
mod tools;
mod unsaved;
mod view_interaction;
mod viewport;
mod viewport_chrome;
mod welcome;
mod window;
mod workspace;

#[cfg(test)]
mod tests;

// The scene helpers are this module's public API: `tests/` and anything else
// outside the editor reaches them here, not at the file they happen to live in.
pub use scene_io::{load_document, load_world, scene_extractor};

use project_panel::state::BrowserState;
use unsaved::Discarding;
use viewport::{RuntimeViewport, SceneRenderers};

/// Which of the editor's two selections the keys act on.
///
/// Set by choosing something rather than by clicking a panel: picking an entity
/// means the keys mean that entity, and picking a file means they mean the
/// file. That is what a selection already communicates, and it needs no
/// separate notion of which panel has focus.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Focus {
    #[default]
    Hierarchy,
    Project,
}

const CAMERA_COMPONENT: &str = CameraComponent::TYPE_NAME;
const SPRITE_COMPONENT: &str = SpriteComponent::TYPE_NAME;
const UI_IMAGE_COMPONENT: &str = UiImageComponent::TYPE_NAME;
const UI_TEXT_COMPONENT: &str = UiTextComponent::TYPE_NAME;
const GRID_NAVIGATION_COMPONENT: &str = GridNavigationComponent::TYPE_NAME;
const GRID_OCCUPANT_COMPONENT: &str = GridOccupantComponent::TYPE_NAME;
const AUDIO_SOURCE_COMPONENT: &str = AudioSourceComponent::TYPE_NAME;
const SCRIPT_COMPONENT: &str = ScriptComponent::TYPE_NAME;
const INITIAL_VIEWPORT_WIDTH: u32 = 960;
const INITIAL_VIEWPORT_HEIGHT: u32 = 540;

/// Which of the two views of the world is being drawn.
///
/// Not which tab is selected — that is the dock's business now — but which
/// *kind* of view a render is for: the Scene view takes camera input and wears
/// editor chrome, and the Game view takes neither.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkspaceTab {
    Scene,
    Game,
}

/// What [`EditorApp::gpu`] hands back: the pieces that share one device.
struct Gpu {
    renderers: SceneRenderers,
    textures: SceneTextures,
    state_for_textures: eframe::egui_wgpu::RenderState,
    scene_viewport: RuntimeViewport,
    game_viewport: RuntimeViewport,
}

struct EditorApp {
    scene: SceneExtractor,
    world: World,
    file: SceneFile,
    /// The history revision the open file was last agreed with.
    ///
    /// Unsaved work is the world having moved away from it, which undoing back
    /// reverses. The flag this replaced was set by every edit and cleared only
    /// by a save, so undoing to exactly the saved state still claimed there was
    /// something to lose.
    saved_revision: u64,
    /// What the user asked for that would throw unsaved work away, waiting on
    /// an answer.
    confirming: Option<Discarding>,
    /// Set once closing has been agreed to, so the close request the editor
    /// cancelled to ask the question is not cancelled a second time.
    closing: bool,
    /// Which entities the editor is pointing at, and which of them the
    /// inspector and the gizmo are about.
    selection: Selection,
    /// What a gizmo drag moves besides its own entity, and where each of them
    /// started. Empty unless a drag on a multiple selection is in progress.
    gizmo_followers: Vec<(EntityId, Transform3D)>,
    /// The entity whose name is being typed into, and the draft.
    ///
    /// Renaming lives on the hierarchy row rather than in a dialog, so this is
    /// the row that has turned into a text field. Editor state, never scene
    /// state: an abandoned rename changes nothing.
    renaming: Option<EntityId>,
    rename_draft: String,
    /// Inspector edits made but not yet written to the world.
    edits: inspector_panel::HeldInspectorEdits,
    /// The scene's name being typed, for the same reason.
    scene_name_edit: Option<String>,
    /// The file the inspector is showing the contents of.
    ///
    /// Beside the slicer rather than inside it: an image and a script are both
    /// "a file the inspector is showing instead of an entity", but what the
    /// panel does with them has nothing in common.
    preview: Option<TextPreview>,
    /// A reusable data profile selected in the project browser.
    profile: Option<ProfileEditor>,
    block_set: Option<crate::block_set::BlockSetEditor>,
    /// The clip the inspector is offering to play, and the device that plays it.
    heard: Option<PathBuf>,
    audition: Audition,
    /// The font the inspector is showing a sample of.
    shown_font: Option<PathBuf>,
    typeface: Typeface,
    /// The asset being renamed in the project browser, and the name so far.
    asset_rename: Option<(PathBuf, String)>,
    /// Which panel the keys that act on "the selection" mean.
    ///
    /// The editor holds two selections — an entity and an asset — and Delete
    /// has to act on one of them. Without this it acted on the entity always,
    /// so a project row's menu could offer a key that did something else to
    /// something else.
    focus: Focus,
    /// The file a delete is waiting to be confirmed for.
    ///
    /// A disk write has no undo behind it, so this is the one browser action
    /// that stops to ask.
    deleting: Option<PathBuf>,
    history: CommandHistory,
    search: String,
    asset_search: String,
    /// The image being sliced, when one was selected in the browser.
    ///
    /// Selecting an asset and selecting an entity are the same act from the
    /// user's side — "show me this" — so they share the inspector and clear
    /// each other rather than fighting over it.
    slicer: Option<Slicer>,
    /// The prefab chosen in the browser, and whether a click places it.
    ///
    /// Beside the slicer because it is the same idea: an asset somebody is
    /// pointing at, with a tool attached to it.
    prefab_brush: Option<crate::prefab::PrefabBrush>,
    /// The tile brush and palette are editor state, not scene state. A map
    /// stores what was painted; it does not store which brush the author last
    /// held or whether the Scene view currently belongs to that brush.
    tilemap_tool: TilemapTool,
    /// The block, height and place/remove mode held by the volume brush.
    tile_volume_tool: TileVolumeTool,
    occlusion: crate::occlusion::OcclusionOverlay,
    /// Clip selection and playback cursor for the inspector's animation
    /// preview. Like runtime animation state, none of this is scene data.
    animation_tool: AnimationTool,
    /// Where the project browser is looking: its selection, the folder it is
    /// scoped to, and what it has folded away.
    ///
    /// Not remembered across launches: which folder you were looking inside is
    /// about the minute rather than the project, unlike the panel sizes beside
    /// it in `Preferences`.
    browser: BrowserState,
    /// The directory the open scene lives in, as it was last read.
    ///
    /// Read when a scene is opened rather than every frame: the browser redraws
    /// at the viewport's frame rate and a directory does not, so a walk per
    /// frame would be a syscall for every row sixty times a second.
    project: ProjectTree,
    /// The project's composed Weave roots, when it has any.
    ///
    /// Loaded when a project is adopted rather than while a viewport draws.
    /// Rendering is allowed to resolve presentation every frame; discovering
    /// and parsing files at frame rate would be a very different and much more
    /// expensive thing.
    styles: ProjectStyles,
    /// Which panel is being dragged, and what one frame measured about where
    /// every group ended up.
    dock: workspace::DockLayout,
    /// The one field that finds anything, and what is typed into it.
    palette: crate::palette::Palette,
    /// How far along the local assistant's setup is.
    assistant: assistant_view::AssistantState,
    preferences: Preferences,
    lifecycle: EngineLifecycle,
    viewport_yaw: f32,
    viewport_pitch: f32,
    viewport_zoom: f32,
    viewport_pan: GlamVec2,
    gizmo_mode: GizmoMode,
    gizmo_space: GizmoSpace,
    gizmo_drag: Option<GizmoDrag>,
    renderers: SceneRenderers,
    /// eframe's device and queue, kept because textures are uploaded whenever a
    /// load completes rather than only while a viewport is being drawn.
    render_state: eframe::egui_wgpu::RenderState,
    /// The textures the open scene draws with, loaded from its own directory.
    textures: SceneTextures,
    thumbnails: thumbnails::Thumbnails,
    /// The Timeline panel's playhead, choice and preview.
    timeline: crate::timeline::TimelineState,
    /// Where each sequence Play is running has got to.
    sequences: sindri_scene::Sequences,
    /// What Play sounds like, and the Audio panel's monitor.
    play_audio: crate::play_audio::PlayAudio,
    /// Where Play's time went, for the Profiler panel.
    profiler: crate::profiler::Profiler,
    /// The Scenes panel's board, and the pictures its cards show.
    scene_board: scene_board_view::SceneBoardState,
    /// The history revision the textures were last asked about.
    ///
    /// An edit can point a mesh at a different texture, and the world is the
    /// only statement of what a scene references, so a change to it is the
    /// signal to ask again. The revision is what makes "changed" cheap to spot.
    textured_revision: TexturedAt,
    scene_viewport: RuntimeViewport,
    game_viewport: RuntimeViewport,
    /// The physics Play steps, and the bodies a scene's colliders became.
    ///
    /// No gravity: the engine has no opinion about which way is down, and a
    /// scene-level setting is a project-format field that arrives with the
    /// feature that reads it. `docs/physics.md` has the open item.
    physics: ScenePhysics2d,
    /// Where the screen elements are and what the pointer is doing to them.
    ///
    /// Recomputed every frame from the world, so a button moved in the
    /// inspector is pressable where it now is rather than where it was.
    screen_ui: ScreenUi,
    /// The run's random stream.
    ///
    /// Put back to its seed every time Play starts, so pressing Play twice
    /// gives the same run twice. That is what makes a bug found in Play a bug
    /// that can be found again, and it is the opposite of what a shipped game
    /// wants — which is why a game seeds itself instead.
    random: sindri_core::Rng,
    /// What a played scene remembers.
    ///
    /// Kept in memory for as long as the editor is open, and never written to
    /// disk. A script's `Save.*` calls work and round-trip inside a session, so
    /// persistence can be play-tested; putting a file into someone's project
    /// directory because they pressed Play would be a side effect they did not
    /// ask for. Where a real save belongs is the shipped host's decision, and
    /// `docs/scripting.md` says so.
    saves: sindri_core::SaveStore,
    /// The fixed-step clock Play runs on.
    ///
    /// The same one a shipped game uses, so a scene steps the same number of
    /// times per second here as it does in the build. Without it a scene
    /// simulated as fast as the editor happened to redraw, which made a
    /// play-test evidence about the editor rather than about the game.
    clock: sindri_core::FixedStepClock,
    /// The live flecks a played scene has thrown.
    ///
    /// Cleared when Play stops, because a fleck outliving the run that threw it
    /// would be a scene at rest that is still moving.
    effects: sindri_scene::Effects2d,
    /// Where the Game view was drawn last frame, in window points.
    ///
    /// Kept because scripts advance before the layout runs, so the rectangle a
    /// pointer is made relative to is the previous frame's. `None` until the
    /// view has been drawn once, and while the workspace is showing the Scene
    /// view instead — a pointer has nowhere to be when the game is not on
    /// screen.
    game_view_rect: Option<egui::Rect>,
    /// The screen shape the Game view is pretending to be.
    ///
    /// Not a preference that outlives the session: it is a thing to look
    /// through while arranging a screen, not a setting about the editor.
    game_device: device::DevicePreview,
    /// Where each animated sprite has got to.
    ///
    /// Runtime state, so it lives here rather than in the world: an animation
    /// playing must not be an unsaved change. Play advances it, pause holds it,
    /// and stop puts every clip back to its first frame.
    animations: SpriteAnimations,
    /// The scripts the open scene runs, and the sources behind them.
    scripts: SceneScripts,
    /// The keyboard a running script reads, translated from egui's.
    input: EditorInput,
    /// The world as it was when Play was pressed.
    ///
    /// Scripts write to the world, which animation never did, so stopping has
    /// to put back what playing changed. Restoring the *authored document*
    /// instead would also throw away every edit made before pressing Play,
    /// which is the author's work rather than the run's.
    play_snapshot: Option<World>,
    /// What the last action the user took had to say, if anything went wrong.
    ///
    /// Kept apart from `render_error` because the two have different lifetimes:
    /// this one stays until something replaces it, while a render result is
    /// recomputed every frame. They shared a field, and the render overwrote a
    /// failed save within one frame of it happening.
    notice: Option<String>,
    /// Whatever the last frame's render reported.
    render_error: Option<String>,
    /// Everything the editor has said, in order.
    console: Console,
    /// The window title as last set.
    ///
    /// Kept so the title is sent when it changes rather than every frame: a
    /// viewport command per frame is sixty round trips a second to say the same
    /// thing.
    title: String,
    /// The welcome window, while it is open.
    ///
    /// Behind an `Arc<Mutex<_>>` because it is drawn by a viewport callback
    /// that egui requires to be `Send + Sync`, and read back here every frame.
    welcome: Option<Arc<Mutex<welcome::Welcome>>>,
    /// Whether the editor's own window has been revealed.
    ///
    /// It starts hidden, so this is also the answer to "is the editor what the
    /// user is looking at" — which decides whether closing the welcome window
    /// closes the editor or just the window.
    window_shown: bool,
    /// The root of the open project, when a project is open.
    ///
    /// A scene can still be edited without one — that is what the editor did
    /// before projects existed, and what a scene named on the command line
    /// outside any project still does.
    open_project_root: Option<PathBuf>,
    /// What the open project calls itself, for the browser's header.
    project_name: Option<String>,
    /// The scene the open project opens on, as its manifest last said.
    ///
    /// Kept beside the name because the browser reads both every frame it
    /// draws, and a manifest read per frame is a file read at the rate a
    /// viewport redraws. The editor is the only thing that changes it.
    project_main_scene: Option<PathBuf>,
    /// The scene a prefab was opened from, and the prefab files edits wrote.
    prefab_session: prefab_writes::PrefabSession,
}

/// What the textures were last asked about: the history revision, since an
/// edit can point a mesh at another texture, and the scripts' prefab
/// revision, since a prefab arriving names textures no edit did.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TexturedAt {
    history: u64,
    prefabs: u64,
}
