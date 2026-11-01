package dev.sindri.mobile

import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * Every designed screen, drawn on the JVM at the prototype's viewport (412×915
 * at one pixel per dp) and written to `app/build/outputs/roborazzi/` under the
 * name of the prototype state it matches, so the two can be set side by side.
 * `mobile/design.md` lists the states and how to capture the prototype's.
 */
@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = "w412dp-h915dp-mdpi", sdk = [35])
class DesignScreens {
    @get:Rule
    val compose = createComposeRule()

    private fun capture(name: String, content: @Composable () -> Unit) {
        compose.setContent { ForgeTheme(content) }
        compose.onRoot().captureRoboImage("build/outputs/roborazzi/$name.png")
    }

    private val lantern = GameCard("Lantern Crossing", "world.scene · 2 scenes · edited yesterday")

    @Test
    fun dashboard() = capture("01-dashboard") {
        DashboardScreen(lantern, {}, {}, {}, {}, {})
    }

    @Test
    fun newGameTemplate() = capture("02-new-template") {
        NewGameScreen(NewGameStep.Template, {}, {}, {}, {}, {})
    }

    @Test
    fun newGameName() = capture("03-new-name") {
        NewGameScreen(NewGameStep.Name("Night Market", Orientation.Portrait), {}, {}, {}, {}, {})
    }

    @Test
    fun newGameCreating() = capture("04-new-creating") {
        NewGameScreen(NewGameStep.Creating("Night Market", 2), {}, {}, {}, {}, {})
    }

    /** The scene behind the chrome: ground, the lamp post by the gap, and Wick. */
    private val canvas: @Composable BoxScope.() -> Unit = {
        TiledSheet(Sheet.Ground, 128f, Modifier.fillMaxSize())
        Sprite(Sheet.Objects, 1, Modifier.offset(150.dp, 60.dp).size(64.dp))
        Sprite(Sheet.Wick, 0, Modifier.offset(170.dp, 540.dp).size(72.dp, 66.dp))
    }

    private fun editor(name: String, ui: EditorUi) = capture(name) { EditorScreen(ui, {}, canvas) }

    private val base = SampleEditor.base
    private val lamp = base.copy(selection = SampleEditor.lampSelection)

    /** The stand-in lamp post's place on the stand-in canvas, as the host would report it. */
    private val lampHandles = HandlesUi(originX = 182f, originY = 122f, left = 166f, top = 64f, right = 198f, bottom = 122f)

    @Test fun editor() = editor("10-editor", base)

    @Test fun moreMenu() = editor("11-more-menu", base.copy(menu = MenuUi(snap = true, lightingPreview = false, tileGrid = false)))

    @Test fun add() = editor("12-add", base.copy(sheet = EditorSheet.Add(SampleEditor.prefabs)))

    @Test fun paint() = editor("13-paint", base.copy(painting = PaintUi()))

    @Test fun assets() = editor("14-assets", base.copy(sheet = EditorSheet.Assets(AssetTab.Prefabs, SampleEditor.assets)))

    @Test fun import() = editor("15-import", base.copy(sheet = EditorSheet.Import(SampleEditor.importFiles)))

    @Test fun layers() = editor("17-layers", base.copy(sheet = EditorSheet.Layers(LayerTab.Layers, SampleEditor.layers, SampleEditor.objects)))

    @Test fun layerObjects() = editor("17b-layers-objects", base.copy(sheet = EditorSheet.Layers(LayerTab.Objects, SampleEditor.layers, SampleEditor.objects)))

    @Test fun scenes() = editor("18-scenes", base.copy(sheet = EditorSheet.Scenes(SampleEditor.scenes, listOf("title" to "world"))))

    @Test fun selected() = editor("19-selected", lamp.copy(handles = lampHandles))

    @Test fun inspectorHalf() = editor("20-inspector-half", lamp.copy(sheet = EditorSheet.Inspector(SampleEditor.lampInspector())))

    /** The engine's lamp as the presenter shows it, to set beside `20-inspector-half`. */
    @Test fun inspectorEngine() = editor(
        "20e-inspector-engine",
        lamp.copy(sheet = EditorSheet.Inspector(InspectorPresenter.present(EngineSamples.lamp, "world.scene", "projects/lamplighter/assets/", details = false).ui)),
    )

    @Test fun inspectorEngineFull() = editor(
        "21e-inspector-engine-full",
        lamp.copy(sheet = EditorSheet.Inspector(InspectorPresenter.present(EngineSamples.lamp, "world.scene", "projects/lamplighter/assets/", details = false).ui, full = true)),
    )

    @Test fun inspectorFull() = editor("21-inspector-full", lamp.copy(sheet = EditorSheet.Inspector(SampleEditor.lampInspector(), full = true)))

    @Test fun behaviourBlocks() = editor("22-behaviour-blocks", lamp.copy(sheet = EditorSheet.Behaviour(SampleEditor.lampBehaviour)))

    @Test fun behaviourCode() = editor("23-behaviour-code", lamp.copy(sheet = EditorSheet.Behaviour(SampleEditor.lampBehaviour, code = true)))

    @Test fun multi() = editor("24-multi", base.copy(selection = SelectionUi(Sym.select_all, "2 selected", "tap to add more", count = 2)))

    @Test fun play() = editor("25-play", base.copy(mode = PlayMode.Play, play = PlayUi(clock = "0:12")))

    /** A thumb down and pushed right and a little up: the stick the game reads as held keys. */
    @Test fun playStick() = editor(
        "25b-play-stick",
        base.copy(mode = PlayMode.Play, play = PlayUi(clock = "0:12", stick = StickUi(anchorX = 206f, anchorY = 700f, knobX = 250f, knobY = 680f))),
    )

    @Test fun paused() = editor("26-frozen", base.copy(mode = PlayMode.Paused, play = PlayUi(clock = "0:12", scrub = 0.8f, recorded = "12 s")))

    @Test fun pausedTune() = editor("26b-frozen-tune", lamp.copy(mode = PlayMode.Paused, sheet = EditorSheet.Inspector(SampleEditor.lampInspector(live = true), live = true)))

    @Test fun review() = editor("27-review", base.copy(sheet = EditorSheet.Review(SampleEditor.runEdits)))

    @Test fun coach() = editor("28-coach", base.copy(coach = "Tap the lamp post next to the gap."))
}
