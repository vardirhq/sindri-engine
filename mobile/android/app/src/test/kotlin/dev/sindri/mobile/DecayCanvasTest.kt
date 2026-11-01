package dev.sindri.mobile

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/**
 * The node canvas at the Pixel 8's density (420 dpi, near its 2.625), with a
 * graph as large as Lamplighter's longest scripts: Gloom's reaches about
 * 26,000 by 14,000 points, which no single drawing surface can be sized to.
 */
@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = "w412dp-h915dp-420dpi", sdk = [35])
class DecayCanvasTest {
    @get:Rule
    val compose = createComposeRule()

    private fun node(key: String, x: Float, y: Float, kind: String = "statement") =
        DecayNode(key, kind, key, null, listOf(DecayPin("to", DecaySlot.Number(1.0, 0 to 1))), listOf(""), x, y)

    @Test
    fun a_graph_as_large_as_the_longest_script_draws_at_phone_density() {
        val graph = DecayGraph(
            nodes = listOf(
                node("Gloom.update", 0f, 0f, kind = "event"),
                node("Gloom.update/0", 310f, 0f),
                node("Gloom.hunt/40", 26000f, 14000f),
            ),
            wires = listOf(
                DecayWire("Gloom.update", "", "Gloom.update/0", "", flow = true),
                DecayWire("Gloom.update/0", "", "Gloom.hunt/40", "", flow = true),
            ),
        )
        compose.setContent {
            ForgeTheme { DecayCanvas(DecayScript("scripts/gloom.decay", "", emptyList(), graph = graph), onEdit = { _, _ -> }, onMove = { _, _, _ -> }) }
        }
        compose.onNodeWithContentDescription("Gloom.update, drag to move, tap for its menu").assertExists()
    }

    @Test
    fun a_node_menu_adds_a_step_from_the_palette_and_deletes() {
        val set = DecayNode("Lamp.start/0", "statement", "Set this.lit", null, emptyList(), listOf(""), 0f, 0f, span = 40 to 56)
        val script = DecayScript(
            "scripts/lamp.decay", "", emptyList(),
            graph = DecayGraph(listOf(set), emptyList()),
            palette = listOf("set:glow" to "Set glow", "comment" to "Comment"),
        )
        val changes = mutableListOf<Triple<String, Pair<Int, Int>, Any?>>()
        compose.setContent {
            ForgeTheme { DecayCanvas(script, onEdit = { _, _ -> }, onMove = { _, _, _ -> }, onChange = { kind, span, value -> changes += Triple(kind, span, value) }) }
        }
        val header = compose.onNodeWithContentDescription("Set this.lit, drag to move, tap for its menu")
        header.performTouchInput { click() }
        compose.onNodeWithText("Add after…").performClick()
        compose.onNodeWithText("Set glow").performClick()
        assertEquals(Triple("insertAfter", 40 to 56, "set:glow"), changes.last())

        header.performTouchInput { click() }
        compose.onNodeWithText("Delete").performClick()
        assertEquals(Triple("delete", 40 to 56, null), changes.last())
    }
}
