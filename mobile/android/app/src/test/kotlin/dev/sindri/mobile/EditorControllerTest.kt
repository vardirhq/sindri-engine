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

        override fun command(command: String, vararg arguments: Any?): String? {
            commands += listOf(command, *arguments)
            when (command) {
                "lighting" -> lighting = arguments[0] as Boolean
                "save" -> state = state.copy(unsaved = false)
            }
            return null
        }

        override fun state() = state.copy(lighting = lighting)
    }

    private val host = Host()
    private val controller = EditorController(host).also { it.opened(density = 2f) }

    @Test
    fun opening_tells_the_host_the_outline_width_and_that_lighting_preview_is_off() {
        assertTrue(listOf("outlineWidth", 3f) in host.commands)
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
}
