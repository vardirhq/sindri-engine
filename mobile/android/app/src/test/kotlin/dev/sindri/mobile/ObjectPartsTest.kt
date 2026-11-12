package dev.sindri.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ObjectPartsTest {
    private fun component(type: String, vararg values: Pair<String, Any?>) = Component(type, type, emptyList(), values = mapOf(*values))

    /** Wick as he is after his art moved onto his body. */
    private val wick = Inspector(
        "Wick",
        listOf(0.0, 0.0, 0.5),
        listOf(
            component("sindri.sprite", "size" to listOf(3.0, 2.75), "offset" to listOf(0.0, 0.0)),
            component("sindri.shadow2d", "contact_size" to listOf(0.9, 0.35), "contact_offset" to listOf(0.0, 0.0)),
            component(
                "sindri.physics2d.collider",
                "pieces" to listOf(
                    mapOf("shape" to mapOf("shape" to "circle", "radius" to 0.4), "offset" to listOf(0.0, 0.45), "rotation" to 0.0),
                    mapOf("shape" to mapOf("shape" to "circle", "radius" to 0.7), "offset" to listOf(0.0, 0.45), "rotation" to 0.0),
                ),
            ),
        ),
        sprite = SpriteRegion("textures/wick.png", "idle-south", listOf(0f, 0f, 0.1f, 0.1f), anchor = listOf(0f, 0.5f)),
    )

    @Test
    fun an_object_lists_every_part_it_can_place() {
        val parts = ObjectParts.of(wick)
        assertEquals(listOf("sprite", "shadow", "collider.0", "collider.1"), parts.map { it.key })
        assertEquals(listOf("Art", "Shadow", "Collider 1", "Collider 2"), parts.map { it.label })
    }

    @Test
    fun the_art_stands_on_its_sheet_anchor_and_moves_by_its_offset() {
        val art = ObjectParts.of(wick).first { it.key == "sprite" }
        assertEquals(Vec2(0.0, 1.375), art.center)
        assertEquals(PartEdit("sindri.sprite", "offset", listOf(0.5, -0.25)), art.moved(art.offset, Vec2(0.5, -0.25)))
    }

    @Test
    fun the_corner_handle_sizes_the_art_about_its_center_and_keeps_a_mirror() {
        val art = ObjectParts.of(wick).first { it.key == "sprite" }
        assertEquals(Vec2(1.5, 1.375 - 1.375), art.handleAt())
        val edit = art.resized(Vec2(2.0, 0.375))!!
        assertEquals("size", edit.path)
        @Suppress("UNCHECKED_CAST")
        val size = edit.value as List<Double>
        assertEquals(4.0, size[0], 1e-9)
        assertEquals(2.0, size[1], 1e-9)
        val mirrored = ObjectParts.of(wick.copy(components = listOf(component("sindri.sprite", "size" to listOf(-3.0, 2.75)))))
            .first().resized(Vec2(2.0, -1.0))!!
        @Suppress("UNCHECKED_CAST")
        assertTrue((mirrored.value as List<Double>)[0] < 0)
    }

    @Test
    fun the_smallest_part_under_a_finger_wins() {
        val parts = ObjectParts.of(wick)
        // On the inner collider circle, which sits inside the outer one and the art.
        assertEquals("collider.0", ObjectParts.at(parts, Vec2(0.0, 0.45), 0.01)?.key)
        // Inside the art but outside both colliders.
        assertEquals("sprite", ObjectParts.at(parts, Vec2(1.2, 2.5), 0.01)?.key)
        assertNull(ObjectParts.at(parts, Vec2(5.0, 5.0), 0.01))
    }

    @Test
    fun a_light_is_found_by_its_center_and_sized_by_its_ring() {
        val lantern = Inspector("Lantern", null, listOf(component("sindri.light2d", "radius" to 4.0, "offset" to listOf(0.2, 0.1))))
        val light = ObjectParts.of(lantern).single()
        assertTrue(light.contains(Vec2(0.2, 0.1), 0.05))
        assertFalse("its reach is not the light", light.contains(Vec2(2.0, 0.1), 0.05))
        assertEquals(Vec2(4.2, 0.1), light.handleAt())
        assertEquals(PartEdit("sindri.light2d", "radius", 2.0), light.resized(Vec2(2.2, 0.1)))
        val (low, high) = ObjectParts.bounds(listOf(light))
        assertTrue("the reach does not stretch the frame", high.x - low.x < 1.0)
    }

    @Test
    fun a_collider_piece_moves_by_its_own_offset_path() {
        val piece = ObjectParts.of(wick).first { it.key == "collider.1" }
        assertEquals(PartEdit("sindri.physics2d.collider", "pieces.1.offset", listOf(0.25, 0.45)), piece.moved(piece.offset, Vec2(0.25, 0.0)))
        assertEquals(1, ObjectParts.colliderPiece("collider.1"))
        assertNull(ObjectParts.colliderPiece("sprite"))
    }

    @Test
    fun a_shadow_with_no_contact_is_not_a_part() {
        val flat = Inspector("Rock", null, listOf(component("sindri.shadow2d", "contact_size" to listOf(0.0, 0.0))))
        assertTrue(ObjectParts.of(flat).isEmpty())
    }
}
