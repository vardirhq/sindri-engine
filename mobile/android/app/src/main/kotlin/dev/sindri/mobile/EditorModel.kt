package dev.sindri.mobile

import androidx.compose.ui.graphics.Color

/**
 * Everything the editor screen draws, as one value, so the screen is a picture
 * of a state rather than a tangle of callbacks. The design is checked by
 * drawing these states (`DesignScreens`); functionality fills them from the
 * engine instead of from [SampleEditor].
 */
data class EditorUi(
    val projectTitle: String,
    val sceneName: String,
    val canUndo: Boolean = false,
    val canRedo: Boolean = false,
    val unsaved: Boolean = false,
    val mode: PlayMode = PlayMode.Edit,
    val sheet: EditorSheet? = null,
    /** The selected object, which swaps the bottom bar for its actions. */
    val selection: SelectionUi? = null,
    val selectingMore: Boolean = false,
    val painting: PaintUi? = null,
    val menu: MenuUi? = null,
    val coach: String? = null,
    val toast: ToastUi? = null,
    /** Where the selection's handles are on the canvas, in canvas pixels. */
    val handles: HandlesUi? = null,
    val play: PlayUi = PlayUi(),
)

enum class PlayMode { Edit, Play, Paused }

/** The selection as the chip above the bar names it. */
data class SelectionUi(val icon: Sym, val name: String, val detail: String, val count: Int = 1)

data class HandlesUi(val x: Float, val y: Float, val width: Float, val height: Float)

data class ToastUi(val text: String, val undo: Boolean = false)

/** The overflow menu's switches and actions. */
data class MenuUi(val snap: Boolean, val lightingPreview: Boolean, val tileGrid: Boolean)

enum class Brush(val label: String) { Path("Path"), Grass("Grass"), Land("Land"), Erase("Erase") }

data class PaintUi(val brush: Brush = Brush.Path, val size: Int = 2, val moveView: Boolean = false)

data class PlayUi(val clock: String = "0:00", val runEdits: List<RunEdit> = emptyList(), val scrub: Float = 1f, val recorded: String = "")

/** A value changed while the game ran, offered for keeping at Stop. */
data class RunEdit(val name: String, val label: String, val from: String, val to: String, val keep: Boolean)

/** Which sheet covers the bottom of the canvas. */
sealed interface EditorSheet {
    data class Add(val prefabs: List<PrefabTile>) : EditorSheet
    data class Assets(val tab: AssetTab, val items: List<AssetItem>) : EditorSheet
    data class Import(val files: List<ImportFile>) : EditorSheet
    data class Layers(val tab: LayerTab, val layers: List<LayerRow>, val objects: List<TreeRow>) : EditorSheet
    data class Scenes(val scenes: List<SceneCard>, val doors: List<Pair<String, String>>) : EditorSheet
    data class Inspector(val inspector: InspectorUi, val full: Boolean = false, val live: Boolean = false) : EditorSheet
    data class Behaviour(val behaviour: BehaviourUi, val code: Boolean = false) : EditorSheet
    data class Review(val edits: List<RunEdit>) : EditorSheet
}

/** A prefab in the Add sheet, drawn from its sprite, [zoom]ed in when it is small in its cell. */
data class PrefabTile(val name: String, val sheet: Sheet, val cell: Int, val zoom: Float = 1f)

enum class AssetTab(val label: String) { Prefabs("Prefabs"), Sprites("Sprites"), Tiles("Tiles"), Scripts("Scripts"), Audio("Audio") }

data class AssetItem(val name: String, val path: String, val sheet: Sheet? = null, val cell: Int = 0, val icon: Sym = Sym.image, val zoom: Float = 1f)

data class ImportFile(val name: String, val detail: String, val supported: Boolean, val sheet: Sheet? = null)

enum class LayerTab(val label: String) { Layers("Layers"), Objects("Objects") }

data class LayerRow(val icon: Sym, val name: String, val detail: String, val visible: Boolean, val locked: Boolean = false)

data class TreeRow(val depth: Int, val icon: Sym, val name: String, val detail: String = "", val chevron: Sym? = null, val selected: Boolean = false)

data class SceneCard(val name: String, val main: Boolean, val current: Boolean, val preview: Boolean)

/** The inspector: a curated view of an object, in sections of friendly rows. */
data class InspectorUi(
    val icon: Sym,
    val name: String,
    val detail: String,
    val active: Boolean,
    val sections: List<SectionUi>,
    val canAddComponent: Boolean = true,
)

data class SectionUi(val icon: Sym, val title: String, val note: String = "", val rows: List<RowUi>)

/** One row of the inspector; each kind is drawn the way the prototype draws it. */
sealed interface RowUi {
    val label: String

    data class Stepper(override val label: String, val axis: String, val axisColor: Color, val value: String) : RowUi
    data class Slider(override val label: String, val fraction: Float, val value: String) : RowUi
    data class Toggle(override val label: String, val on: Boolean) : RowUi
    data class Segment(override val label: String, val options: List<String>, val chosen: Int) : RowUi
    data class Swatches(override val label: String, val colors: List<Color>, val chosen: Int) : RowUi
    data class SpriteRef(override val label: String, val sheet: Sheet, val cell: Int, val file: String, val detail: String) : RowUi
    data class Behaviour(override val label: String) : RowUi
    data class Facts(override val label: String, val items: List<Pair<String, String>>) : RowUi
    data class More(override val label: String, val expanded: Boolean) : RowUi
}

/** Behaviour, as the blocks show it: rules of When → Do → And, and the Decay they make. */
data class BehaviourUi(
    val subject: String,
    val templates: List<BehaviourTemplate>,
    val rules: List<RuleUi>,
    val codeFile: String,
    val code: List<String>,
    val codeNote: String,
)

data class BehaviourTemplate(val icon: Sym, val label: String, val description: String, val chosen: Boolean)

data class RuleUi(val whenIcon: Sym, val whenText: String, val actions: List<ActionUi>, val adding: Boolean = false)

data class ActionUi(val icon: Sym, val text: String, val target: String? = null, val value: String? = null)

/** What a tap or gesture on the editor asks for. The screen only reports. */
sealed interface EditorEvent {
    data object Home : EditorEvent
    data object OpenScenes : EditorEvent
    data object Undo : EditorEvent
    data object Redo : EditorEvent
    data object Save : EditorEvent
    data object ToggleMenu : EditorEvent
    data class MenuToggle(val item: MenuItem) : EditorEvent
    data object FrameAll : EditorEvent
    data class Tool(val tool: BarItem) : EditorEvent
    data object Play : EditorEvent
    data object Stop : EditorEvent
    data object PauseOrResume : EditorEvent
    data object StepBack : EditorEvent
    data object StepOnce : EditorEvent
    data object CloseSheet : EditorEvent
    data object ToggleDetent : EditorEvent
    data object OpenInspector : EditorEvent
    data object OpenBehaviour : EditorEvent
    data object DismissCoach : EditorEvent
    data object ToastUndo : EditorEvent
    data class Brush(val brush: dev.sindri.mobile.Brush) : EditorEvent
    data class BrushSize(val size: Int) : EditorEvent
    data object ToggleMoveView : EditorEvent
    data object DonePainting : EditorEvent
    data class AssetTabChosen(val tab: AssetTab) : EditorEvent
    data object OpenImport : EditorEvent
    data object BackToAssets : EditorEvent
    data class LayerTabChosen(val tab: LayerTab) : EditorEvent
    data class LayerVisibility(val layer: String) : EditorEvent
    data class TreeRowTapped(val row: TreeRow) : EditorEvent
    data class SceneTapped(val scene: SceneCard) : EditorEvent
    data object NewScene : EditorEvent
    data class PrefabChosen(val prefab: PrefabTile) : EditorEvent
    data class InspectorRow(val section: String, val row: String, val change: RowChange) : EditorEvent
    data object ToggleActive : EditorEvent
    data object AddComponent : EditorEvent
    data object BehaviourCode : EditorEvent
    data object BehaviourBlocks : EditorEvent
    data class KeepRunEdit(val index: Int) : EditorEvent
    data object KeepEdits : EditorEvent
    data object DiscardEdits : EditorEvent
}

enum class MenuItem { Snap, LightingPreview, TileGrid }

/** How an inspector row was changed. */
sealed interface RowChange {
    data object Minus : RowChange
    data object Plus : RowChange
    data class To(val fraction: Float) : RowChange
    data class Choose(val index: Int) : RowChange
    data class Switch(val on: Boolean) : RowChange
    data object Open : RowChange
}

/** The bottom bar's buttons, with nothing selected and with a selection. */
enum class BarItem(val label: String, val icon: Sym, val danger: Boolean = false) {
    Add("Add", Sym.add_circle),
    Paint("Paint", Sym.brush),
    Assets("Assets", Sym.photo_library),
    Layers("Layers", Sym.layers),
    Scenes("Scenes", Sym.view_quilt),
    Properties("Properties", Sym.tune),
    Behaviour("Behaviour", Sym.conversion_path),
    Duplicate("Duplicate", Sym.content_copy),
    SelectMore("Select more", Sym.library_add),
    Group("Group", Sym.folder_special),
    Delete("Delete", Sym.delete, danger = true),
}
