package dev.sindri.mobile

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * Turns editor events into [EditorUi], and the native host's state into what
 * the design shows.
 *
 * What the engine does is the engine's: the scene, picking, moving, field
 * edits (presented the design's way by [InspectorPresenter]), undo and redo,
 * and saving, which happens after every change as the
 * prototype implies (it has no Save button). Everything else is the design's
 * sample content until it is wired, and says so with a toast where acting on
 * it would otherwise pretend. `mobile/design.md` lists which is which.
 */
class EditorController(
    private val native: Native,
    /** Where the open project's assets are among the app's own, for sprite thumbnails. */
    private val assetRoot: String = "projects/lamplighter/assets/",
) {
    /** The native calls the controller needs, so the design tests can run without the library. */
    interface Native {
        fun command(command: String, vararg arguments: Any?): String?
        fun state(): EditorState
    }

    var ui by mutableStateOf(SampleEditor.base.copy(projectTitle = "Lamplighter", canUndo = false))
        private set

    /** The last native state, which selection and the inspector are drawn from. */
    private var state = EditorState()
    private var error: String? = null

    /**
     * The overflow menu's switches. Kept apart from whether the menu is open,
     * so closing it forgets nothing. Lighting preview starts off, as the
     * design has it.
     */
    private var settings = MenuUi(snap = true, lightingPreview = false, tileGrid = false)

    /** Screen pixels per dp, which turns the host's pixels into the design's dp. */
    private var density = 1f

    /** The inspector as last presented, with what each of its rows edits. */
    private var presented: InspectorPresenter.Presented? = null

    /** Whether the inspector's Details section is open. */
    private var details = false

    /**
     * A scene was opened on the host: tells it what the design draws at this
     * screen's [density] (the outline's width) and the lighting preview
     * setting, and draws the state it opened in.
     */
    fun opened(density: Float) {
        this.density = density
        native.command("density", density)
        native.command("lighting", settings.lightingPreview)
        refresh()
    }

    /**
     * Something on the canvas changed: a tap, a drag, a pinch. [settled] is a
     * finger lifting, which ends a move, so an edit it made is saved then, as
     * every other edit is.
     */
    fun canvasChanged(settled: Boolean = false) {
        refresh()
        if (settled && state.unsaved) {
            error = native.command("save")
            refresh()
        }
    }

    /** Something changed on the host: draws its state again. */
    fun refresh() {
        state = native.state()
        val inspector = state.inspector
        val selection = inspector?.let { SelectionUi(InspectorPresenter.iconFor(it), it.name, "Properties") }
        settings = settings.copy(lightingPreview = state.lighting)
        val play = state.play
        val stoppedItself = play == null && ui.mode != PlayMode.Edit
        if (stoppedItself) {
            // The game failed and the host stopped it: say why, and clear it.
            error = state.playError ?: "The game stopped."
            native.command("stop")
        }
        ui = ui.copy(
            mode = if (stoppedItself) PlayMode.Edit else ui.mode,
            play = if (play == null) PlayUi() else ui.play.copy(clock = clock(play.seconds), stick = stick(play)),
            canUndo = state.undo != null,
            canRedo = state.redo != null,
            unsaved = state.unsaved,
            selection = selection,
            handles = handles(),
            menu = ui.menu?.let { settings },
            sheet = when (val sheet = ui.sheet) {
                is EditorSheet.Inspector -> inspector?.let { sheet.copy(inspector = inspectorUi(it)) }
                is EditorSheet.Behaviour -> if (inspector == null) null else sheet
                else -> sheet
            },
            toast = error?.let { ToastUi(it) } ?: ui.toast,
        )
        error = null
    }

    fun on(event: EditorEvent, onHome: () -> Unit) {
        when (event) {
            EditorEvent.Home -> onHome()
            EditorEvent.Undo -> nativeEdit("undo")
            EditorEvent.Redo -> nativeEdit("redo")
            EditorEvent.Save -> nativeEdit("save")
            EditorEvent.ToggleMenu -> ui = ui.copy(menu = if (ui.menu == null) settings else null)
            is EditorEvent.MenuToggle -> menuToggle(event.item)
            EditorEvent.FrameAll -> notWired("Frame everything")
            EditorEvent.OpenScenes -> toggle(EditorSheet.Scenes(SampleEditor.scenes, listOf("title" to "world")))
            is EditorEvent.Tool -> tool(event.tool)
            EditorEvent.Play -> play()
            EditorEvent.Stop -> stop()
            EditorEvent.PauseOrResume -> {
                val pausing = ui.mode == PlayMode.Play
                error = native.command("pause", pausing)
                ui = ui.copy(mode = if (pausing) PlayMode.Paused else PlayMode.Play, sheet = null)
                refresh()
            }
            EditorEvent.StepOnce -> {
                error = native.command("stepOnce")
                refresh()
            }
            EditorEvent.StepBack -> notWired("Stepping back")
            EditorEvent.CloseSheet -> ui = ui.copy(sheet = null)
            EditorEvent.ToggleDetent -> (ui.sheet as? EditorSheet.Inspector)?.let { ui = ui.copy(sheet = it.copy(full = !it.full)) }
            EditorEvent.OpenInspector -> openInspector()
            EditorEvent.OpenBehaviour -> openBehaviour()
            EditorEvent.BehaviourCode -> (ui.sheet as? EditorSheet.Behaviour)?.let { ui = ui.copy(sheet = it.copy(code = true)) }
            EditorEvent.BehaviourBlocks -> (ui.sheet as? EditorSheet.Behaviour)?.let { ui = ui.copy(sheet = it.copy(code = false)) }
            EditorEvent.DismissCoach -> ui = ui.copy(coach = null)
            EditorEvent.ToastUndo -> nativeEdit("undo")
            is EditorEvent.Brush -> ui.painting?.let { ui = ui.copy(painting = it.copy(brush = event.brush)) }
            is EditorEvent.BrushSize -> ui.painting?.let { ui = ui.copy(painting = it.copy(size = event.size)) }
            EditorEvent.ToggleMoveView -> ui.painting?.let { ui = ui.copy(painting = it.copy(moveView = !it.moveView)) }
            EditorEvent.DonePainting -> ui = ui.copy(painting = null)
            is EditorEvent.AssetTabChosen -> ui = ui.copy(sheet = EditorSheet.Assets(event.tab, if (event.tab == AssetTab.Prefabs) SampleEditor.assets else emptyList()))
            EditorEvent.OpenImport -> ui = ui.copy(sheet = EditorSheet.Import(SampleEditor.importFiles))
            EditorEvent.BackToAssets -> ui = ui.copy(sheet = EditorSheet.Assets(AssetTab.Prefabs, SampleEditor.assets))
            is EditorEvent.LayerTabChosen -> (ui.sheet as? EditorSheet.Layers)?.let { ui = ui.copy(sheet = it.copy(tab = event.tab)) }
            is EditorEvent.InspectorRow -> inspectorRow(event)
            is EditorEvent.HandleTaken -> {
                error = native.command("handle", event.handle.native, event.x, event.y, settings.snap)
                refresh()
            }
            is EditorEvent.HandleDragged -> {
                error = native.command("drag", event.x, event.y)
                refresh()
            }
            EditorEvent.HandleReleased -> nativeEdit("release")
            else -> notWired(null)
        }
    }

    /**
     * Flips one of the menu's switches. Lighting preview is the host's: the
     * canvas draws the scene's 2D lights only while it is on. Snap keeps the
     * handles to whole steps. The tile grid is not drawn yet.
     */
    private fun menuToggle(item: MenuItem) {
        settings = when (item) {
            MenuItem.Snap -> settings.copy(snap = !settings.snap)
            MenuItem.LightingPreview -> settings.copy(lightingPreview = !settings.lightingPreview)
            MenuItem.TileGrid -> settings.copy(tileGrid = !settings.tileGrid)
        }
        when (item) {
            MenuItem.LightingPreview -> {
                error = native.command("lighting", settings.lightingPreview)
                refresh()
            }
            MenuItem.TileGrid -> ui = ui.copy(menu = settings, toast = ToastUi("The tile grid is not drawn yet."))
            MenuItem.Snap -> ui = ui.copy(menu = settings)
        }
    }

    /**
     * Plays the scene for real. The game is opened from the project's files,
     * which hold every edit because each is saved as it is made; one that
     * failed to save is saved first, and if it still will not, Play says so
     * rather than run something older than what is on screen.
     */
    private fun play() {
        if (native.state().unsaved) {
            native.command("save")?.let { failed ->
                ui = ui.copy(toast = ToastUi("Play needs the scene saved first: $failed"))
                return
            }
        }
        val failed = native.command("play")
        if (failed != null) {
            ui = ui.copy(toast = ToastUi("The game would not start: $failed"))
            return
        }
        ui = ui.copy(mode = PlayMode.Play, sheet = null, menu = null, painting = null)
        refresh()
    }

    /** Stops the game. The scene is as it was when Play was pressed: the game never touched it. */
    private fun stop() {
        error = native.command("stop")
        ui = ui.copy(mode = PlayMode.Edit, sheet = null, play = PlayUi())
        refresh()
    }

    private fun clock(seconds: Double): String {
        val whole = seconds.toLong().coerceAtLeast(0)
        return "${whole / 60}:${(whole % 60).toString().padStart(2, '0')}"
    }

    private fun stick(play: PlayState): StickUi? {
        val (anchorX, anchorY) = play.anchor ?: return null
        val (knobX, knobY) = play.knob ?: return null
        return StickUi(anchorX / density, anchorY / density, knobX / density, knobY / density)
    }

    /** The host's selection origin and bounds, in the design's dp. */
    private fun handles(): HandlesUi? {
        val (x, y) = state.origin ?: return null
        val bounds = state.bounds?.takeIf { it.size == 4 } ?: return null
        return HandlesUi(x / density, y / density, bounds[0] / density, bounds[1] / density, bounds[2] / density, bounds[3] / density)
    }

    private fun tool(tool: BarItem) {
        when (tool) {
            BarItem.Add -> toggle(EditorSheet.Add(SampleEditor.prefabs))
            BarItem.Paint -> ui = ui.copy(painting = PaintUi(), sheet = null, toast = ToastUi("Painting the ground is not wired yet."))
            BarItem.Assets -> toggle(EditorSheet.Assets(AssetTab.Prefabs, SampleEditor.assets))
            BarItem.Layers -> toggle(EditorSheet.Layers(LayerTab.Layers, SampleEditor.layers, SampleEditor.objects))
            BarItem.Scenes -> toggle(EditorSheet.Scenes(SampleEditor.scenes, listOf("title" to "world")))
            BarItem.Properties -> openInspector()
            BarItem.Behaviour -> openBehaviour()
            else -> notWired(tool.label)
        }
    }

    /** Opens [sheet], or closes it if the same kind is already open, as the bar does. */
    private fun toggle(sheet: EditorSheet) {
        ui = ui.copy(sheet = if (ui.sheet?.javaClass == sheet.javaClass) null else sheet, menu = null)
    }

    private fun openInspector() {
        val inspector = state.inspector ?: return
        ui = ui.copy(sheet = EditorSheet.Inspector(inspectorUi(inspector)), menu = null)
    }

    /** The engine's selection as the design's inspector, remembering what each row edits. */
    private fun inspectorUi(inspector: Inspector): InspectorUi =
        InspectorPresenter.present(inspector, ui.sceneName, assetRoot, details).also { presented = it }.ui

    private fun openBehaviour() {
        val inspector = state.inspector ?: return
        ui = ui.copy(
            sheet = EditorSheet.Behaviour(
                BehaviourUi(
                    subject = inspector.name,
                    templates = listOf(
                        BehaviourTemplate(Sym.star, "Collectible", "Disappears and adds to the score", false),
                        BehaviourTemplate(Sym.door_front, "Door to a scene", "Interact to go to another scene", false),
                    ),
                    rules = emptyList(),
                    codeFile = "No script yet",
                    code = emptyList(),
                    codeNote = "Behaviour blocks are not wired to Decay yet.",
                ),
            ),
        )
    }

    /**
     * A change to an inspector row, made through what the presenter bound the
     * row to. A slider's values are applied as the finger moves and merged
     * into one step, which is saved when it lifts; anything else is one step,
     * saved at once.
     */
    private fun inspectorRow(event: EditorEvent.InspectorRow) {
        val binding = presented?.binding(event.section, event.row) ?: return notWired(event.row)
        when {
            binding is RowBinding.ToggleDetails -> {
                details = !details
                refresh()
            }
            binding is RowBinding.NotWired -> notWired(binding.what)
            event.change == RowChange.Settled -> nativeEdit("settle")
            event.change is RowChange.To -> binding.edit(event.change)?.let { edit ->
                error = native.command(edit.command, *edit.arguments.toTypedArray())
                refresh()
            }
            else -> binding.edit(event.change)?.let { edit -> nativeEdit(edit.command, *edit.arguments.toTypedArray()) } ?: notWired(event.row)
        }
    }

    /** Runs an edit on the engine and saves it, as the design has no Save button. */
    private fun nativeEdit(command: String, vararg arguments: Any?) {
        error = native.command(command, *arguments)
        if (error == null && command != "save" && native.state().unsaved) {
            error = native.command("save")
        }
        refresh()
    }

    private fun notWired(what: String?) {
        ui = ui.copy(toast = ToastUi(if (what != null) "$what is not wired yet." else "Not wired yet."))
    }

    /**
     * The system Back gesture, which steps out one level at a time: the menu,
     * then a sheet, then painting, then a running game, then the selection.
     * `false` when there is nothing left to step out of, and Back leaves the
     * editor.
     */
    fun back(): Boolean {
        when {
            ui.menu != null -> ui = ui.copy(menu = null)
            ui.sheet != null -> ui = ui.copy(sheet = null)
            ui.painting != null -> ui = ui.copy(painting = null)
            ui.mode != PlayMode.Edit -> stop()
            ui.selection != null -> {
                error = native.command("deselect")
                refresh()
            }
            else -> return false
        }
        return true
    }

    fun dismissToast() {
        ui = ui.copy(toast = null)
    }
}
