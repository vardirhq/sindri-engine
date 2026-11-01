package dev.sindri.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * What the controller asks of the host, against a host that only records:
 * the menu's switches, the selection's handles, and saving after a drag.
 */
class EditorControllerTest {
    /** Records each command and answers from [state], as the native host would. */
    private class Host : EditorController.Native {
        val commands = mutableListOf<List<Any?>>()
        var state = EditorState()
        var lighting = false

        var playing: PlayState? = null

        override fun command(command: String, vararg arguments: Any?): String? {
            commands += listOf(command, *arguments)
            when (command) {
                "lighting" -> lighting = arguments[0] as Boolean
                "save" -> state = state.copy(unsaved = false)
                "play" -> playing = PlayState(seconds = 0.0, paused = false)
                "stop" -> playing = null
                "pause" -> playing = playing?.copy(paused = arguments[0] as Boolean)
            }
            return null
        }

        override fun state() = state.copy(lighting = lighting, play = playing)

        override fun script() = EngineSamples.lampScript
    }

    private val host = Host()
    private val controller = EditorController(host).also { it.opened(density = 2f) }

    @Test
    fun opening_tells_the_host_the_density_and_that_lighting_preview_is_off() {
        assertTrue(listOf("density", 2f) in host.commands)
        assertTrue(listOf("lighting", false) in host.commands)
        assertFalse(host.lighting)
    }

    @Test
    fun lighting_preview_reaches_the_host_and_the_menu_remembers_it() {
        controller.on(EditorEvent.ToggleMenu) {}
        assertEquals(false, controller.ui.menu?.lightingPreview)
        controller.on(EditorEvent.MenuToggle(MenuItem.LightingPreview)) {}
        assertTrue("the canvas is drawn lit", host.lighting)
        assertEquals(true, controller.ui.menu?.lightingPreview)

        controller.on(EditorEvent.ToggleMenu) {}
        assertNull(controller.ui.menu)
        controller.on(EditorEvent.ToggleMenu) {}
        assertEquals("reopening keeps it on", true, controller.ui.menu?.lightingPreview)

        controller.on(EditorEvent.MenuToggle(MenuItem.LightingPreview)) {}
        assertFalse(host.lighting)
        controller.on(EditorEvent.MenuToggle(MenuItem.Snap)) {}
        controller.on(EditorEvent.ToggleMenu) {}
        controller.on(EditorEvent.ToggleMenu) {}
        assertEquals(false, controller.ui.menu?.snap)
    }

    @Test
    fun the_handles_are_placed_in_dp_and_a_handle_drag_snaps_and_saves() {
        host.state = EditorState(origin = 200f to 300f, bounds = listOf(160f, 120f, 240f, 300f))
        controller.refresh()
        assertEquals(HandlesUi(100f, 150f, 80f, 60f, 120f, 150f), controller.ui.handles)

        controller.on(EditorEvent.HandleTaken(HandleKind.Turn, 200f, 40f)) {}
        assertEquals(listOf("handle", "turn", 200f, 40f, true), host.commands.last())
        controller.on(EditorEvent.HandleDragged(260f, 60f)) {}
        assertEquals(listOf("drag", 260f, 60f), host.commands.last())
        host.state = host.state.copy(unsaved = true)
        controller.on(EditorEvent.HandleReleased) {}
        assertEquals(listOf("save"), host.commands.last())

        host.state = EditorState()
        controller.refresh()
        assertNull("nothing selected, no handles", controller.ui.handles)
    }

    @Test
    fun a_move_on_the_canvas_is_saved_when_the_finger_lifts() {
        host.state = EditorState(unsaved = true)
        controller.canvasChanged(settled = false)
        assertFalse(listOf("save") in host.commands)
        controller.canvasChanged(settled = true)
        assertNotNull(host.commands.lastOrNull { it == listOf("save") })
        assertFalse(controller.ui.unsaved)
    }

    @Test
    fun back_lets_go_of_the_sheet_then_the_selection_then_leaves() {
        host.state = EditorState(inspector = Inspector("Lamp post", listOf(0.0, 0.0, 0.0), emptyList()))
        controller.refresh()
        controller.on(EditorEvent.OpenInspector) {}
        assertNotNull(controller.ui.sheet)

        assertTrue(controller.back())
        assertNull("the sheet closes first", controller.ui.sheet)
        assertNotNull(controller.ui.selection)

        host.state = EditorState()
        assertTrue(controller.back())
        assertEquals(listOf("deselect"), host.commands.last())
        assertNull("then the selection is let go", controller.ui.selection)

        assertFalse("then Back leaves the editor", controller.back())
    }

    @Test
    fun play_saves_first_runs_pauses_and_stop_returns_to_editing() {
        host.state = EditorState(unsaved = true)
        controller.on(EditorEvent.Play) {}
        val saved = host.commands.indexOf(listOf("save"))
        assertTrue("an unsaved edit is saved before the game opens", saved in 0 until host.commands.indexOf(listOf("play")))
        assertEquals(PlayMode.Play, controller.ui.mode)

        host.playing = host.playing?.copy(seconds = 75.4, anchor = 200f to 400f, knob = 260f to 400f)
        controller.refresh()
        assertEquals("1:15", controller.ui.play.clock)
        assertEquals(StickUi(100f, 200f, 130f, 200f), controller.ui.play.stick)

        controller.on(EditorEvent.PauseOrResume) {}
        assertEquals(listOf("pause", true), host.commands.last())
        assertEquals(PlayMode.Paused, controller.ui.mode)

        controller.on(EditorEvent.Stop) {}
        assertEquals(PlayMode.Edit, controller.ui.mode)
        assertNull(host.playing)
    }

    @Test
    fun a_game_that_stops_itself_returns_to_editing_and_says_why() {
        controller.on(EditorEvent.Play) {}
        host.playing = null
        host.state = host.state.copy(playError = "The game stopped: a solver failed")
        controller.refresh()
        assertEquals(PlayMode.Edit, controller.ui.mode)
        assertEquals("The game stopped: a solver failed", controller.ui.toast?.text)
        assertEquals(listOf("stop"), host.commands.last())
    }

    @Test
    fun a_script_opens_as_decay_blocks_and_a_slot_edit_changes_the_file_there() {
        host.state = EditorState(
            inspector = Inspector(
                "Lamp post", listOf(0.0, 0.0, 0.0),
                listOf(Component("sindri.script", "Script", emptyList(), values = mapOf("source" to "scripts/lamp.decay"))),
            ),
        )
        controller.refresh()
        controller.on(EditorEvent.OpenBehaviour) {}
        assertEquals(listOf("openScript", "scripts/lamp.decay"), host.commands.last())
        val sheet = controller.ui.sheet as EditorSheet.Behaviour
        assertEquals(EngineSamples.lampScript, sheet.behaviour.decay)

        controller.on(EditorEvent.ScriptSlot(DecaySlot.Number(1.5, 40 to 43), 2.0)) {}
        assertEquals(listOf("editScript", "number", 40, 43, 2.0), host.commands.last())

        controller.on(EditorEvent.MoveNode("Lamp.update/0", 320f, 96f)) {}
        assertEquals(listOf("moveNode", "Lamp.update/0", 320f, 96f), host.commands.last())
        assertTrue("the sheet stays open on the script", controller.ui.sheet is EditorSheet.Behaviour)

        controller.on(EditorEvent.ChangeScript("insertInside", 120 to 180, mapOf("template" to "if:lit", "branch" to 0))) {}
        assertEquals(listOf("editScript", "insertInside", 120, 180, mapOf("template" to "if:lit", "branch" to 0)), host.commands.last())
    }
}
