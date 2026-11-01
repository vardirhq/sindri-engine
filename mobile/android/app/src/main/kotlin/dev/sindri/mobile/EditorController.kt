package dev.sindri.mobile

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.Color
import java.util.Locale

/**
 * Turns editor events into [EditorUi], and the native host's state into what
 * the design shows.
 *
 * What the engine does is the engine's: the scene, picking, moving, field
 * edits, undo and redo, and saving, which happens after every change as the
 * prototype implies (it has no Save button). Everything else is the design's
 * sample content until it is wired, and says so with a toast where acting on
 * it would otherwise pretend. `mobile/design.md` lists which is which.
 */
class EditorController(private val native: Native) {
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

    /** Something on the canvas changed: a tap, a drag, an undo. */
    fun refresh() {
        state = native.state()
        val inspector = state.inspector
        val selection = inspector?.let { SelectionUi(iconFor(it), it.name, "Properties") }
        ui = ui.copy(
            canUndo = state.undo != null,
            canRedo = state.redo != null,
            unsaved = state.unsaved,
            selection = selection,
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
            EditorEvent.ToggleMenu -> ui = ui.copy(menu = if (ui.menu == null) MenuUi(snap = true, lightingPreview = false, tileGrid = false) else null)
            is EditorEvent.MenuToggle -> ui.menu?.let { menu ->
                ui = ui.copy(
                    menu = when (event.item) {
                        MenuItem.Snap -> menu.copy(snap = !menu.snap)
                        MenuItem.LightingPreview -> menu.copy(lightingPreview = !menu.lightingPreview)
                        MenuItem.TileGrid -> menu.copy(tileGrid = !menu.tileGrid)
                    },
                )
            }
            EditorEvent.FrameAll -> notWired("Frame everything")
            EditorEvent.OpenScenes -> toggle(EditorSheet.Scenes(SampleEditor.scenes, listOf("title" to "world")))
            is EditorEvent.Tool -> tool(event.tool)
            EditorEvent.Play -> ui = ui.copy(mode = PlayMode.Play, sheet = null, menu = null, toast = ToastUi("Play runs the real game in slice step 4."))
            EditorEvent.Stop -> ui = ui.copy(mode = PlayMode.Edit, sheet = null)
            EditorEvent.PauseOrResume -> ui = ui.copy(mode = if (ui.mode == PlayMode.Play) PlayMode.Paused else PlayMode.Play, sheet = null)
            EditorEvent.StepBack, EditorEvent.StepOnce -> notWired("Stepping")
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
            else -> notWired(null)
        }
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

    private fun inspectorRow(event: EditorEvent.InspectorRow) {
        val component = state.inspector?.components?.firstOrNull { it.display == event.section } ?: return notWired(event.row)
        val field = component.fields.firstOrNull { it.name == event.row } ?: return notWired(event.row)
        val value: Any = when (val change = event.change) {
            RowChange.Minus -> step(field, -1)
            RowChange.Plus -> step(field, +1)
            is RowChange.Switch -> change.on
            else -> return notWired(event.row)
        }
        nativeEdit("setField", component.type, field.name, value)
    }

    private fun step(field: Field, direction: Int): Any {
        val current = (field.value as Number).toDouble()
        val next = current + direction * field.step
        return if (field.isWhole) Math.round(next) else Math.round(next * 1000.0) / 1000.0
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

    fun dismissToast() {
        ui = ui.copy(toast = null)
    }

    private fun iconFor(inspector: Inspector): Sym {
        val types = inspector.components.map { it.type }
        return when {
            "sindri.light2d" in types -> Sym.emoji_objects
            "sindri.camera" in types -> Sym.camera_alt
            "sindri.tilemap" in types -> Sym.grid_view
            "sindri.physics2d.rigid_body" in types -> Sym.directions_walk
            "sindri.sprite" in types -> Sym.image
            else -> Sym.deployed_code
        }
    }

    /** The engine's components as the design's sections: numbers step, switches toggle. */
    private fun inspectorUi(inspector: Inspector): InspectorUi {
        val place = inspector.position?.let { position ->
            SectionUi(
                Sym.open_with, "Place", "drag to move",
                listOf(
                    RowUi.Stepper("Across", "X", Color(0xFFD6605C), format(position[0])),
                    RowUi.Stepper("Down", "Y", Color(0xFF7FB069), format(position[1])),
                ),
            )
        }
        val components = inspector.components.filter { it.fields.isNotEmpty() }.map { component ->
            SectionUi(
                Sym.extension, component.display, component.type,
                component.fields.map { field ->
                    if (field.isSwitch) {
                        RowUi.Toggle(field.name, field.value as Boolean)
                    } else {
                        RowUi.Stepper(field.name, "", Forge.TextFaint, format((field.value as Number).toDouble()))
                    }
                },
            )
        }
        return InspectorUi(iconFor(inspector), inspector.name, "In ${ui.sceneName}", active = true, sections = listOfNotNull(place) + components, canAddComponent = false)
    }

    private fun format(value: Double): String =
        if (value == Math.rint(value)) value.toLong().toString() else String.format(Locale.ROOT, "%.2f", value).trimEnd('0').trimEnd('.')
}
