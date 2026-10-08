//! How the editor starts: deciding what to open, building the window's GPU
//! state, and assembling the app around the first scene.
//!
//! Its own file because the constructor is the one part of the app that runs
//! once, and it had grown to a third of the module that defines the app.

use glam::Vec2 as GlamVec2;
use sindri_core::{CommandHistory, World};
use sindri_scene::SceneExtractor;

use crate::audition::Audition;
use crate::selection::Selection;
use crate::typeface::Typeface;
use crate::{
    animation::AnimationTool,
    console::Console,
    gizmo::{GizmoMode, GizmoSpace},
    input::EditorInput,
    preferences::Preferences,
    project::{Launch, ProjectTree, launch},
    scene_file::SceneFile,
    scripts::SceneScripts,
    textures::SceneTextures,
    tile_volume::TileVolumeTool,
    tilemap::TilemapTool,
    weave_styles::ProjectStyles,
};

use super::project_panel::state::BrowserState;
use super::runtime::initialized_lifecycle;
use super::scene_io::{load_world, open_named_scene, scene_extractor};
use super::viewport::{RuntimeViewport, SceneRenderers};
use super::{
    EditorApp, Focus, Gpu, TexturedAt, assistant_view, device, inspector_panel, scene_board_view,
    thumbnails, workspace,
};

impl EditorApp {
    /// What this launch opens, before anything else is built.
    ///
    /// Its own step because deciding is one thing and constructing is another,
    /// and the constructor had grown past what a reader can hold.
    fn opening(preferences: &Preferences) -> (Launch, SceneFile, Option<String>) {
        // Only a scene opens a file here — opening a project needs the editor
        // that this is building.
        let decided = launch::decide(
            std::env::args().nth(1).as_deref(),
            preferences.recent_projects.most_recent(),
            preferences.open_last_project,
        );
        let (file, open_error) = match &decided {
            Launch::Scene(path) => open_named_scene(&path.display().to_string()),
            Launch::Project(_) | Launch::Welcome => (
                SceneFile::detached(sindri_core::SceneDocument::default()),
                None,
            ),
        };
        (decided, file, open_error)
    }

    /// Everything the GPU side of the editor needs, built together.
    ///
    /// One step because they are one concern and they share one device: two
    /// viewports, the renderers they draw through, and the texture set they
    /// resolve against all hang off the render state, and separating them in
    /// the constructor only separated the lines, not the coupling.
    fn gpu(context: &eframe::CreationContext<'_>, scene: Option<&std::path::Path>) -> Gpu {
        let render_state = context
            .wgpu_render_state
            .clone()
            .expect("the native editor requires eframe's WGPU renderer");
        let renderers = SceneRenderers::new(&render_state);
        let textures = SceneTextures::for_scene(&render_state.device, &render_state.queue, scene);
        let state_for_textures = render_state.clone();
        let scene_viewport = RuntimeViewport::new(render_state.clone(), "Sindri editor scene view");
        let game_viewport = RuntimeViewport::new(render_state.clone(), "Sindri editor game view");
        Gpu {
            renderers,
            textures,
            state_for_textures,
            scene_viewport,
            game_viewport,
        }
    }

    /// The world the editor starts with, and what went wrong reaching it.
    ///
    /// A scene that will not load must not take the editor down with it. This
    /// used to unwrap inside the constructor, so a file that parsed and then
    /// failed validation killed the process before the window existed — and the
    /// failure it unwrapped was one the editor should not have had in the first
    /// place.
    fn opening_world(scene: &SceneExtractor, file: &SceneFile) -> (World, Option<String>) {
        match load_world(scene, file) {
            Ok(world) => (world, None),
            Err(error) => (World::default(), Some(error)),
        }
    }

    // Long because it names every field the editor holds, once; the work it
    // does is the handful of calls after the literal.
    #[allow(clippy::too_many_lines)]
    pub(super) fn new(
        context: &eframe::CreationContext<'_>,
        benchmark: Option<crate::benchmark::BenchmarkPlan>,
    ) -> Self {
        crate::ui::theme::install(&context.egui_ctx);
        // A benchmark measures the editor as it ships, not as somebody last
        // arranged it.
        let preferences = if benchmark.is_some() {
            Preferences::default()
        } else {
            Preferences::load(context.storage)
        };
        let scene = scene_extractor();
        let (decided, file, open_error) = Self::opening(&preferences);
        let (world, load_error) = Self::opening_world(&scene, &file);
        let Gpu {
            renderers,
            textures,
            state_for_textures,
            scene_viewport,
            game_viewport,
        } = Self::gpu(context, file.anchor());
        let project = ProjectTree::beside(file.anchor());
        let mut app = Self {
            game_scene: scene.clone(),
            scene,
            world,
            file,
            saved_revision: 0,
            confirming: None,
            closing: false,
            // Nothing is selected until something is chosen. This used to name
            // an entity from the demo scene, which selected the cube in that
            // one scene and silently nothing in every other.
            selection: Selection::default(),
            gizmo_followers: Vec::new(),
            renaming: None,
            rename_draft: String::new(),
            edits: inspector_panel::HeldInspectorEdits::default(),
            scene_name_edit: None,
            preview: None,
            profile: None,
            block_set: None,
            heard: None,
            audition: Audition::default(),
            shown_font: None,
            typeface: Typeface::default(),
            asset_rename: None,
            focus: Focus::Hierarchy,
            deleting: None,
            history: CommandHistory::default(),
            search: String::new(),
            asset_search: String::new(),
            slicer: None,
            prefab_brush: None,
            tilemap_tool: TilemapTool::default(),
            tile_volume_tool: TileVolumeTool::default(),
            occlusion: crate::occlusion::OcclusionOverlay::default(),
            animation_tool: AnimationTool::default(),
            browser: BrowserState::default(),
            project,
            styles: ProjectStyles::default(),
            dock: workspace::DockLayout::default(),
            palette: crate::palette::Palette::default(),
            assistant: assistant_view::AssistantState::default(),
            preferences,
            lifecycle: initialized_lifecycle(),
            viewport_yaw: 0.0,
            viewport_pitch: 0.0,
            viewport_zoom: 1.0,
            viewport_pan: GlamVec2::ZERO,
            gizmo_mode: GizmoMode::Select,
            gizmo_space: GizmoSpace::Local,
            gizmo_drag: None,
            renderers,
            render_state: state_for_textures,
            textures,
            thumbnails: thumbnails::Thumbnails::default(),
            scene_board: scene_board_view::SceneBoardState::default(),
            profiler: crate::profiler::Profiler::default(),
            play_audio: crate::play_audio::PlayAudio::new(crate::play_audio::native()),
            sheet_camera: super::sprite_sheet_view::SheetCamera::default(),
            timeline: crate::timeline::TimelineState::default(),
            textured_revision: TexturedAt::default(),
            scene_viewport,
            game_viewport,
            game_view_rect: None,
            last_game_view: None,
            game_device: device::DevicePreview::default(),
            saves: super::runtime::EditorSaves::default(),
            session: None,
            still: super::Stillness::default(),
            clock: fixed_step_clock(),
            scripts: SceneScripts::for_scene(None),
            input: EditorInput::default(),
            play_snapshot: None,
            notice: open_error.or(load_error),
            render_error: None,
            console: Console::default(),
            title: String::new(),
            welcome: None,
            window_shown: false,
            open_project_root: None,
            project_name: None,
            project_main_scene: None,
            prefab_session: super::prefab_writes::PrefabSession::default(),
            benchmark: benchmark.map(|plan| {
                let opened = std::env::args().nth(1).unwrap_or_default();
                super::benchmark::BenchmarkRun::new(plan, opened)
            }),
            disk_watch: super::wake::DiskWatch::start(context.egui_ctx.clone()),
        };
        // A benchmark plays at the screen the standalone benchmark draws,
        // so both lay the game out against the same media queries.
        if app.benchmark.is_some() {
            app.game_device = super::device::DevicePreview {
                name: "Benchmark",
                size: Some((1280.0, 720.0)),
            };
        }
        // Said after the field is built rather than during it, because what
        // there is to say is read off the world and the bindings.
        if let Some(failure) = app.notice.clone() {
            app.console.error(failure);
        }
        app.arrange_for(decided);
        app.request_first_assets();
        app.remember_open_scene();
        app
    }

    /// Puts the editor where the launch said it should be.
    ///
    /// Only a scene has been opened by the time this runs. A project is opened
    /// through the same path the welcome window opens one through, so a launch
    /// and a click arrange the editor identically rather than in two places
    /// that have to be kept agreeing.
    fn arrange_for(&mut self, decided: Launch) {
        match decided {
            Launch::Scene(path) => {
                self.announce_scene();
                self.adopt_project_for(&path);
                self.project = self.project_tree();
            }
            Launch::Project(root) => self.open_project_at(&root),
            Launch::Welcome => self.open_welcome(),
        }
    }
}

impl EditorApp {
    /// Asks for the opening scene's textures and scripts, and says what is
    /// missing.
    fn request_first_assets(&mut self) {
        let notes = self.textures.request(&self.world, &mut self.renderers.text);
        self.record_texture_notes(notes);
        self.reload_scripts();
    }
}

/// The clock Play steps by, at the engine's default rate.
fn fixed_step_clock() -> sindri_core::FixedStepClock {
    sindri_core::FixedStepClock::new(sindri_core::FixedStepConfig::default())
        .expect("the default fixed-step configuration is valid")
}
