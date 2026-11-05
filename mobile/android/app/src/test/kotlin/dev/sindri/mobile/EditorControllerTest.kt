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
        var scriptEdits = 0
        var openSceneError: String? = null
        var addError: String? = null

        override fun command(command: String, vararg arguments: Any?): String? {
            commands += listOf(command, *arguments)
            if (command == "openScene" && openSceneError != null) return openSceneError
            if (command.startsWith("add") && addError != null) return addError
            when (command) {
                "lighting" -> lighting = arguments[0] as Boolean
                "openScene" -> state = state.copy(sceneName = arguments[0] as String)
                "save" -> state = state.copy(unsaved = false)
                "play" -> playing = PlayState(seconds = 0.0, paused = false)
                "stop" -> playing = null
                "pause" -> playing = playing?.copy(paused = arguments[0] as Boolean)
                "editScript" -> scriptEdits += if (arguments[0] == "undo") -1 else 1
            }
            return null
        }

        override fun state() = state.copy(lighting = lighting, play = playing)

        override fun script() = EngineSamples.lampScript.copy(canUndo = scriptEdits > 0)
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
    fun object_tree_moves_a_child_and_saves() {
        controller.on(EditorEvent.Tool(BarItem.Layers)) {}
        controller.on(EditorEvent.MoveTree("child")) {}
        val sheet = controller.ui.sheet as EditorSheet.Layers
        assertEquals("child", sheet.moving)
        host.state = host.state.copy(unsaved = true)
        controller.on(EditorEvent.TreeRowTapped(TreeRow(0, Sym.folder, "Parent", id = "parent"))) {}
        assertTrue(listOf("reparentObject", "child", "parent") in host.commands)
        assertTrue(listOf("save") in host.commands)
        assertNull((controller.ui.sheet as EditorSheet.Layers).moving)
    }

    @Test
    fun frame_everything_closes_the_menu_and_does_not_save() {
        controller.on(EditorEvent.ToggleMenu) {}
        controller.on(EditorEvent.FrameAll) {}
        assertNull(controller.ui.menu)
        assertEquals(listOf("frameAll"), host.commands.last())
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
    fun backgrounding_finishes_an_edit_and_saves_it() {
        host.state = EditorState(unsaved = true)
        controller.background()
        assertEquals(listOf("release"), host.commands[host.commands.size - 3])
        assertEquals(listOf("settle"), host.commands[host.commands.size - 2])
        assertEquals(listOf("save"), host.commands.last())
        assertFalse(controller.ui.unsaved)
    }

    @Test
    fun backgrounding_a_clean_scene_does_not_save() {
        controller.background()
        assertFalse(listOf("save") in host.commands)
    }

    @Test
    fun selection_actions_reach_the_host_and_edits_are_saved() {
        host.state = EditorState(inspector = Inspector("Lamp", null, emptyList()), selectionCount = 1)
        controller.refresh()
        controller.on(EditorEvent.Tool(BarItem.SelectMore)) {}
        assertEquals(listOf("selectMore"), host.commands.last())
        host.state = host.state.copy(unsaved = true, selectionCount = 2)
        controller.on(EditorEvent.Tool(BarItem.Duplicate)) {}
        assertTrue(listOf("duplicate") in host.commands)
        assertEquals(listOf("save"), host.commands.last())
        host.state = host.state.copy(unsaved = true)
        controller.on(EditorEvent.Tool(BarItem.Delete)) {}
        assertTrue(listOf("delete") in host.commands)
        assertEquals(listOf("save"), host.commands.last())
    }

    @Test
    fun layers_list_real_objects_and_select_the_tapped_handle() {
        val objects = listOf(TreeRow(0, Sym.web_asset, "HUD", detail = "Screen UI", id = "4294967296"))
        host.state = EditorState(objects = objects)
        controller.on(EditorEvent.Tool(BarItem.Layers)) {}
        val sheet = controller.ui.sheet as EditorSheet.Layers
        assertEquals(objects, sheet.objects)
        assertTrue(sheet.layers.isEmpty())
        controller.on(EditorEvent.TreeRowTapped(objects.single())) {}
        assertTrue(listOf("selectObject", "4294967296") in host.commands)
        assertNull(controller.ui.sheet)
    }

    @Test
    fun object_marker_coordinates_and_drag_reach_the_host() {
        host.state = EditorState(gizmos = listOf(GizmoUi("42", "Light", "light", 200f, 300f, false)))
        controller.refresh()
        assertEquals(100f, controller.ui.gizmos.single().x, 0.001f)
        assertEquals(150f, controller.ui.gizmos.single().y, 0.001f)
        controller.on(EditorEvent.GizmoTaken("42", 200f, 300f)) {}
        assertTrue(listOf("gizmoPress", "42", 200f, 300f) in host.commands)
        controller.on(EditorEvent.HandleDragged(240f, 310f)) {}
        assertEquals(listOf("drag", 240f, 310f), host.commands.last())
        host.state = host.state.copy(unsaved = true)
        controller.on(EditorEvent.HandleReleased) {}
        assertEquals(listOf("save"), host.commands.last())
    }

    @Test
    fun add_lists_project_prefabs_and_sends_the_selected_asset() {
        val prefab = PrefabTile("marker", id = "prefabs/marker.prefab")
        host.state = EditorState(prefabs = listOf(prefab))
        controller.on(EditorEvent.Tool(BarItem.Add)) {}
        assertEquals(listOf(prefab), (controller.ui.sheet as EditorSheet.Add).prefabs)
        host.state = host.state.copy(unsaved = true)
        controller.on(EditorEvent.PrefabChosen(prefab)) {}
        assertTrue(listOf("addPrefab", "prefabs/marker.prefab") in host.commands)
        assertEquals(listOf("save"), host.commands.last())
        assertNull(controller.ui.sheet)
        controller.on(EditorEvent.Tool(BarItem.Add)) {}
        controller.on(EditorEvent.AddBasic("light2d")) {}
        assertTrue(listOf("addBasic", "light2d") in host.commands)
    }

    @Test
    fun failed_placement_keeps_the_add_sheet_open() {
        val prefab = PrefabTile("marker", id = "prefabs/marker.prefab")
        host.state = EditorState(prefabs = listOf(prefab))
        host.addError = "Prefab could not be placed"
        controller.on(EditorEvent.Tool(BarItem.Add)) {}
        controller.on(EditorEvent.PrefabChosen(prefab)) {}
        assertNotNull(controller.ui.sheet)
        assertEquals("Prefab could not be placed", controller.ui.toast?.text)
    }

    @Test
    fun scenes_come_from_the_host_and_open_the_tapped_scene() {
        val opened = mutableListOf<String>()
        val controller = EditorController(host, onSceneOpened = { opened += it })
        val world = SceneCard("world.scene", false, true, false)
        val title = SceneCard("title.scene", true, false, false)
        host.state = EditorState(sceneName = "world.scene", scenes = listOf(title, world))
        controller.opened(2f)
        controller.on(EditorEvent.OpenScenes) {}
        assertEquals(listOf(title, world), (controller.ui.sheet as EditorSheet.Scenes).scenes)
        controller.on(EditorEvent.SceneTapped(title)) {}
        assertEquals(listOf("openScene", "title.scene"), host.commands.last())
        assertEquals("title.scene", controller.ui.sceneName)
        assertEquals(listOf("title.scene"), opened)
        assertNull(controller.ui.sheet)
    }

    @Test
    fun failed_scene_switch_keeps_the_sheet_and_does_not_remember_the_target() {
        val opened = mutableListOf<String>()
        val controller = EditorController(host, onSceneOpened = { opened += it })
        val target = SceneCard("title.scene", true, false, false)
        host.state = EditorState(sceneName = "world.scene", scenes = listOf(target), unsaved = true)
        controller.opened(2f)
        controller.on(EditorEvent.OpenScenes) {}
        host.openSceneError = "Save failed"
        controller.on(EditorEvent.SceneTapped(target)) {}
        assertEquals("world.scene", controller.ui.sceneName)
        assertTrue(opened.isEmpty())
        assertNotNull(controller.ui.sheet)
        assertEquals("Save failed", controller.ui.toast?.text)
    }

    @Test
    fun recovery_conflict_offers_explicit_choices() {
        host.state = EditorState(recoveryConflict = true)
        controller.refresh()
        assertTrue(controller.recoveryConflict)
        controller.chooseRecovery(true)
        assertEquals(listOf("recoveryRestore"), host.commands.last())
        controller.chooseRecovery(false)
        assertEquals(listOf("recoveryDiscard"), host.commands.last())
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
    fun add_component_uses_the_host_and_saves() {
        host.state = EditorState(inspector = Inspector("Object", null, emptyList(), addable = listOf(ComponentChoice("sindri.physics2d.collider", "Collider"))), selectionCount = 1)
        controller.refresh()
        controller.on(EditorEvent.OpenInspector) {}
        controller.on(EditorEvent.AddComponent) {}
        assertTrue((controller.ui.sheet as EditorSheet.Inspector).adding)
        host.state = host.state.copy(unsaved = true)
        controller.on(EditorEvent.ComponentChosen("sindri.physics2d.collider")) {}
        assertTrue(listOf("addComponent", "sindri.physics2d.collider") in host.commands)
        assertTrue(listOf("save") in host.commands)
        assertFalse((controller.ui.sheet as EditorSheet.Inspector).adding)
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
    fun undo_with_a_script_open_takes_back_its_edits_and_otherwise_the_scenes() {
        host.state = EditorState(
            inspector = Inspector(
                "Lamp post", listOf(0.0, 0.0, 0.0),
                listOf(Component("sindri.script", "Script", emptyList(), values = mapOf("source" to "scripts/lamp.decay"))),
            ),
        )
        controller.refresh()
        controller.on(EditorEvent.OpenBehaviour) {}
        assertFalse("nothing to take back in a script just opened", controller.ui.canUndo)

        controller.on(EditorEvent.ScriptSlot(DecaySlot.Number(1.5, 40 to 43), 2.0)) {}
        assertTrue(controller.ui.canUndo)
        controller.refresh()
        assertTrue("the scene's history does not hide the script's", controller.ui.canUndo)

        controller.on(EditorEvent.Undo) {}
        assertEquals(listOf("editScript", "undo", 0, 0, null), host.commands.last())
        assertFalse(controller.ui.canUndo)

        controller.on(EditorEvent.CloseSheet) {}
        controller.on(EditorEvent.Undo) {}
        assertEquals(listOf("undo"), host.commands.last())
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
