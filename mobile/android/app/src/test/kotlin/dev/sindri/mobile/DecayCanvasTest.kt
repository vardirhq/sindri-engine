package dev.sindri.mobile

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTouchInput
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
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
                node("Gloom.update/40", 26000f, 14000f),
            ),
            wires = listOf(
                DecayWire("Gloom.update", "", "Gloom.update/0", "", flow = true),
                DecayWire("Gloom.update/0", "", "Gloom.update/40", "", flow = true),
            ),
        )
        compose.setContent {
            ForgeTheme { DecayCanvas(DecayScript("scripts/gloom.decay", "", emptyList(), graph = graph), onEdit = { _, _ -> }, onMove = { _, _, _ -> }) }
        }
        compose.onNodeWithContentDescription("Gloom.update, drag to move, tap for its menu").assertExists()
        compose.onNodeWithContentDescription("Gloom.update/40, drag to move, tap for its menu").assertExists()
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

    @Test
    fun steps_read_in_running_order_with_branches_indented_under_their_statement() {
        fun at(key: String, outputs: List<String> = listOf("")) = DecayNode(key, "statement", key, null, emptyList(), outputs, 0f, 0f)
        val start = DecayNode("start", "event", "When it starts", null, emptyList(), listOf(""), 0f, 0f)
        val branch = at("if", listOf("True", "False", "Next"))
        val (yes, no, after) = listOf(at("yes"), at("no"), at("after"))
        fun flow(from: String, out: String, to: String) = DecayWire(from, out, to, "", flow = true)
        val graph = DecayGraph(
            listOf(start, branch, yes, no, after),
            listOf(flow("start", "", "if"), flow("if", "True", "yes"), flow("if", "False", "no"), flow("if", "Next", "after")),
        )
        assertEquals(
            listOf(StepRow.Step(branch, 0), StepRow.Step(yes, 1), StepRow.Branch("Otherwise", 0), StepRow.Step(no, 1), StepRow.Step(after, 0)),
            steps(graph, start),
        )
    }

    @Test
    fun calls_read_in_words_and_the_finder_matches_words_or_code() {
        assertEquals("Play sound", plain("Audio.play"))
        assertEquals("Set number · Save", plain("Save.set_number"))
        assertEquals("Is held? · Ui", plain("Ui.is_held"))
        assertEquals("Set this.light", plain("Set this.light"))
        assertEquals("If", plain("If"))

        val palette = listOf("set:glow" to "Set glow", "comment" to "Comment")
        val engine = listOf("engine:Audio.play" to "Audio.play(\"\", 0)", "engine:World.find" to "World.find(\"\")")
        assertEquals(4, findSteps("", palette, engine).size)
        assertEquals(listOf("engine:Audio.play"), findSteps("sound", palette, engine).map { it.first })
        assertEquals(listOf("engine:World.find"), findSteps("world.f", palette, engine).map { it.first })
        assertEquals(listOf("set:glow"), findSteps("glow", palette, engine).map { it.first })
    }

    @Test
    fun finder_categories_and_search_cover_every_result() {
        val palette = listOf("return" to "Return")
        val engine = (1..75).map { "engine:World.call$it" to "World.call$it(this.entity)" } +
            ("engine:World.despawn" to "World.despawn(this.entity)") +
            ("engine:Audio.play" to "Audio.play(\"\", 0)")
        assertTrue("Objects" in stepCategories(palette, engine))
        assertTrue("Audio" in stepCategories(palette, engine))
        assertEquals(76, findSteps("", palette, engine, "Objects").size)
        assertEquals(listOf("engine:World.despawn"), findSteps("despawn", palette, engine).map { it.first })
        assertEquals(listOf("return"), findSteps("", palette, engine, "Script").map { it.first })
    }

    @Test
    fun finder_searches_engine_calls_in_the_node_menu() {
        val set = DecayNode("Lamp.start/0", "statement", "Set this.lit", null, emptyList(), listOf(""), 0f, 0f, span = 40 to 56)
        val script = DecayScript(
            "scripts/lamp.decay", "", emptyList(),
            graph = DecayGraph(listOf(set), emptyList()),
            engine = listOf("engine:World.despawn" to "World.despawn(this.entity)"),
        )
        val changes = mutableListOf<Triple<String, Pair<Int, Int>, Any?>>()
        compose.setContent {
            ForgeTheme { DecayCanvas(script, onEdit = { _, _ -> }, onMove = { _, _, _ -> }, onChange = { kind, span, value -> changes += Triple(kind, span, value) }) }
        }
        compose.onNodeWithContentDescription("Set this.lit, drag to move, tap for its menu").performTouchInput { click() }
        compose.onNodeWithText("Add after…").performClick()
        compose.onNodeWithText("Search").performClick()
        compose.onNodeWithTag("step-search").performTextInput("despawn")
        compose.onNodeWithText("Remove entity  ·  World.despawn(this.entity)").performClick()
        assertEquals(Triple("insertAfter", 40 to 56, "engine:World.despawn"), changes.last())
    }

    @Test
    fun object_pins_keep_their_span_and_offer_compatible_fields() {
        val slot = DecaySlot.Name("this.entity", 10 to 21, objectReference = true)
        assertTrue(slot.editable)
        val script = DecayScript.read("""{"name":"s.decay","blocks":{"containers":[{"name":"S","fields":[{"name":"target","type":"Entity"},{"name":"speed","type":"number"}],"handlers":[]}]},"graph":{"nodes":[{"key":"n","inputs":[{"name":"entity","value":{"kind":"name","path":"this.entity","span":[10,21]}}]}]}}""")!!
        val actual = script.graph.nodes.single().inputs.single().value as DecaySlot.Name
        assertEquals(10 to 21, actual.span)
        assertTrue(actual.objectReference)
        assertEquals(listOf("target"), script.fieldsFor(actual))
    }

    @Test
    fun object_dialog_selects_a_named_scene_object() {
        val slot = DecaySlot.Name("this.entity", 10 to 21, objectReference = true)
        val script = DecayScript("s.decay", "", emptyList(), objects = listOf("Wick"))
        val choices = mutableListOf<Pair<String, String>>()
        compose.setContent {
            ForgeTheme { ObjectDialog(script, slot, onDone = {}) { kind, value -> choices += kind to value } }
        }
        compose.onNodeWithText("Wick").performClick()
        assertEquals(listOf("findObject" to "Wick"), choices)
        compose.onNodeWithText("This object").performClick()
        assertEquals("useField" to "entity", choices.last())
    }

    @Test
    fun the_plus_offers_only_the_events_a_script_lacks() {
        fun handler(name: String) = DecayHandler(name, name, event = true, params = emptyList(), body = emptyList(), note = null)
        fun script(vararg names: String) = DecayScript("s.decay", "", listOf(DecayContainer("script", "S", emptyList(), names.map(::handler))))
        assertEquals(listOf("start" to "When it starts", "update" to "Every frame"), missingEvents(script()))
        assertEquals(listOf("update" to "Every frame"), missingEvents(script("start", "ignite")))
        assertEquals(emptyList<Pair<String, String>>(), missingEvents(script("start", "update")))
    }

    @Test
    fun a_pin_offers_the_fields_of_its_type() {
        val script = DecayScript(
            "scripts/lamp.decay", "",
            listOf(
                DecayContainer(
                    "script", "Lamp",
                    listOf(
                        DecayField("glow", exported = true, mutable = false, type = "number", value = DecaySlot.Number(1.5, 40 to 43), note = null),
                        DecayField("lit", exported = false, mutable = true, type = "bool", value = DecaySlot.Bool(false, 60 to 65), note = null),
                        DecayField("label", exported = false, mutable = true, type = "String", value = DecaySlot.Text("", 70 to 72), note = null),
                    ),
                    emptyList(),
                ),
            ),
        )
        assertEquals(listOf("glow"), script.fieldsFor(DecaySlot.Number(3.0, 90 to 93)))
        assertEquals(listOf("label"), script.fieldsFor(DecaySlot.Text("hi", 90 to 94)))
        assertEquals(emptyList<String>(), script.fieldsFor(DecaySlot.Bool(true, 90 to 94)))
    }

    @Test
    fun one_handler_shows_at_a_time_and_its_tab_switches() {
        val graph = DecayGraph(
            nodes = listOf(
                node("Lamp.start", 0f, 0f, kind = "event"),
                node("Lamp.start/0", 310f, 0f),
                node("Lamp.update", 0f, 300f, kind = "event"),
                node("Lamp.update/0", 310f, 300f),
                node("Lamp.update/0:to", 40f, 360f, kind = "value"),
            ),
            wires = listOf(DecayWire("Lamp.update/0:to", "", "Lamp.update/0", "to", flow = false)),
        )
        compose.setContent {
            ForgeTheme { DecayCanvas(DecayScript("scripts/lamp.decay", "", emptyList(), graph = graph), onEdit = { _, _ -> }, onMove = { _, _, _ -> }) }
        }
        compose.onNodeWithContentDescription("Lamp.start/0, drag to move, tap for its menu").assertExists()
        compose.onNodeWithContentDescription("Lamp.update/0, drag to move, tap for its menu").assertDoesNotExist()
        compose.onNodeWithText("Lamp.update").performClick()
        compose.onNodeWithContentDescription("Lamp.update/0, drag to move, tap for its menu").assertExists()
        compose.onNodeWithContentDescription("Lamp.start/0, drag to move, tap for its menu").assertDoesNotExist()
    }
}
