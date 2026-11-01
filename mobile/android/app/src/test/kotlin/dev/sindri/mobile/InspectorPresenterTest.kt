package dev.sindri.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The engine's selection presented as the design's sections, and what each row writes back. */
class InspectorPresenterTest {
    private val presented = InspectorPresenter.present(EngineSamples.lamp, "world.scene", "projects/lamplighter/assets/", details = false)

    private fun section(title: String) = presented.ui.sections.first { it.title == title }

    private fun row(section: String, label: String) = section(section).rows.first { it.label == label }

    @Test
    fun the_sections_are_the_designs_in_its_order_then_the_rest_then_behaviour_and_details() {
        assertEquals(
            listOf("Place", "Light 2D", "Sprite", "Collider 2D", "Camera Behaviour", "Behaviour", "Details"),
            presented.ui.sections.map { it.title },
        )
        assertEquals(Sym.emoji_objects, presented.ui.icon)
    }

    @Test
    fun place_reads_in_tiles_degrees_and_times_and_writes_the_transform() {
        assertEquals("10", (row("Place", "Across") as RowUi.Stepper).value)
        assertEquals("1.5×", (row("Place", "Size") as RowUi.Slider).value)
        assertEquals(NativeEdit("setPlace", listOf("across", 10.5, false)), presented.binding("Place", "Across")!!.edit(RowChange.Plus))
        assertEquals(NativeEdit("setPlace", listOf("turn", 90.0, true)), presented.binding("Place", "Turn")!!.edit(RowChange.To(0.75f)))
        assertEquals(NativeEdit("setPlace", listOf("mirror", false, false)), presented.binding("Sprite", "Mirror")!!.edit(RowChange.Switch(false)))
        assertEquals(true, (row("Sprite", "Mirror") as RowUi.Toggle).on)
    }

    @Test
    fun a_light_has_its_sliders_and_swatches_and_a_drag_merges() {
        val radius = row("Light 2D", "Radius") as RowUi.Slider
        assertEquals("4.5 tiles", radius.value)
        val edit = presented.binding("Light 2D", "Radius")!!.edit(RowChange.To(1f))!!
        assertEquals(listOf("sindri.light2d", "radius", 12.0, true), edit.arguments)
        val color = row("Light 2D", "Color") as RowUi.Swatches
        assertEquals("the lamp's color is the first preset", 0, color.chosen)
        assertEquals(3, color.colors.size)
        assertEquals(
            NativeEdit("setField", listOf("sindri.light2d", "color", listOf(0.67, 0.78, 1.0, 1.0), false)),
            presented.binding("Light 2D", "Color")!!.edit(RowChange.Choose(1)),
        )
        assertEquals(listOf("sindri.light2d", "shadows", true, false), presented.binding("Light 2D", "Casts shadows")!!.edit(RowChange.Switch(true))!!.arguments)
    }

    @Test
    fun the_sprite_is_thumbnailed_from_its_sheet_region() {
        val sprite = row("Sprite", "Sprite") as RowUi.SpriteRef
        assertEquals("objects.png", sprite.file)
        assertEquals("lamp-post-lit · layer 3", sprite.detail)
        assertEquals(ProjectImage("projects/lamplighter/assets/textures/objects.png", listOf(1f / 11, 0f, 1f / 11, 1f)), sprite.image)
        assertNull(sprite.sheet)
        assertEquals(listOf("sindri.sprite", "layer", 4L, false), presented.binding("Sprite", "Draw layer")!!.edit(RowChange.Plus)!!.arguments)
    }

    @Test
    fun a_collider_shape_change_writes_the_shape_its_fields_need() {
        assertEquals(0, (row("Collider 2D", "Shape") as RowUi.Segment).chosen)
        assertEquals(
            listOf("sindri.physics2d.collider", "pieces.0.shape", mapOf("shape" to "box", "half_extents" to listOf(1.2, 1.2)), false),
            presented.binding("Collider 2D", "Shape")!!.edit(RowChange.Choose(1))!!.arguments,
        )
        assertEquals(listOf("sindri.physics2d.collider", "pieces.0.sensor", false, false), presented.binding("Collider 2D", "Sensor (doesn’t block)")!!.edit(RowChange.Switch(false))!!.arguments)
    }

    @Test
    fun a_component_with_no_presentation_is_shown_plainly_and_details_open() {
        assertTrue(section("Camera Behaviour").rows.any { it is RowUi.Toggle && it.label == "follow" })
        assertEquals(RowBinding.ToggleDetails, presented.binding("Details", "Show all details"))
        val open = InspectorPresenter.present(EngineSamples.lamp, "world.scene", "", details = true)
        val facts = open.ui.sections.last().rows.filterIsInstance<RowUi.Facts>().single().items.toMap()
        assertEquals("lamp-3", facts["Stable ID"])
        assertEquals("sindri.light2d", facts["Light 2D"])
    }
}
