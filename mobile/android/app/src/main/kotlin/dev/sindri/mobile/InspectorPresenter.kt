package dev.sindri.mobile

import androidx.compose.ui.graphics.Color
import java.util.Locale
import kotlin.math.abs
import kotlin.math.roundToLong

/**
 * Presents the engine's selection the way the design's inspector does: a few
 * friendly sections (Place in tiles, Light 2D, Sprite, Collider 2D, Body,
 * Shadow) with sliders over sensible ranges, units and color swatches, rather
 * than each component's raw fields. Components it has no presentation of are
 * shown plainly after them, so nothing the engine holds is hidden.
 *
 * Every row it draws carries a [RowBinding]: the native edit a change to that
 * row makes. Ranges are the design's, narrowed to what the engine's own field
 * meanings allow, so a slider never offers a value the engine would refuse.
 */
object InspectorPresenter {
    /** The design's inspector for [inspector], and what each of its rows edits. */
    data class Presented(val ui: InspectorUi, val bindings: Map<String, RowBinding>) {
        fun binding(section: String, row: String): RowBinding? = bindings[key(section, row)]
    }

    fun key(section: String, row: String) = "$section/$row"

    /** Colors a light can be given in one tap: the design's lamp, cool and ember. */
    val lightColors = listOf(
        listOf(1.0, 0.84, 0.59),
        listOf(0.67, 0.78, 1.0),
        listOf(1.0, 0.59, 0.35),
    )

    private const val LIGHT = "sindri.light2d"
    private const val SPRITE = "sindri.sprite"
    private const val COLLIDER = "sindri.physics2d.collider"
    private const val BODY = "sindri.physics2d.rigid_body"
    private const val SHADOW = "sindri.shadow2d"
    private const val OCCLUDER = "sindri.light2d.occluder"
    private const val SCRIPT = "sindri.script"
    private const val TAGS = "sindri.tags"

    /** The components given sections of their own, in the order they are shown. */
    private val presented = setOf(LIGHT, SPRITE, COLLIDER, BODY, SHADOW, OCCLUDER, SCRIPT, TAGS)

    /**
     * [inspector] as the design's sections. [assetRoot] is where the project's
     * assets are among the app's own, for the sprite's thumbnail; [details]
     * opens the Details section.
     */
    fun present(inspector: Inspector, sceneName: String, assetRoot: String, details: Boolean): Presented {
        val bindings = mutableMapOf<String, RowBinding>()
        fun SectionBuilder.done() = SectionUi(icon, title, note, rows).also { section ->
            rowBindings.forEach { (row, binding) -> bindings[key(section.title, row)] = binding }
        }

        val sections = listOfNotNull(
            place(inspector)?.done(),
            inspector.component(LIGHT)?.let { light(it, inspector.tile).done() },
            inspector.component(SPRITE)?.let { sprite(it, inspector, assetRoot).done() },
            inspector.component(COLLIDER)?.let { collider(it, inspector.tile).done() },
            inspector.component(BODY)?.let { body(it).done() },
            inspector.component(SHADOW)?.let { shadow(it).done() },
            inspector.component(OCCLUDER)?.let { occluder(it).done() },
        ) + inspector.components.filter { it.type !in presented }.mapNotNull { plain(it)?.done() } +
            listOfNotNull(inspector.component(SCRIPT)?.let { behaviour(it).done() }, details(inspector, details).done())

        val icon = iconFor(inspector)
        return Presented(InspectorUi(icon, inspector.name, "In $sceneName", active = true, sections = sections, canAddComponent = false), bindings)
    }

    fun iconFor(inspector: Inspector): Sym {
        val types = inspector.components.map { it.type }
        return when {
            LIGHT in types -> Sym.emoji_objects
            "sindri.camera" in types -> Sym.camera_alt
            "sindri.tilemap" in types -> Sym.grid_view
            BODY in types -> Sym.directions_walk
            SPRITE in types -> Sym.image
            else -> Sym.deployed_code
        }
    }

    /** One section being put together: its rows, and what each row edits. */
    private class SectionBuilder(val icon: Sym, val title: String, val note: String = "") {
        val rows = mutableListOf<RowUi>()
        val rowBindings = mutableListOf<Pair<String, RowBinding>>()

        fun add(row: RowUi, binding: RowBinding? = null) {
            rows += row
            if (binding != null) rowBindings += row.label to binding
        }
    }

    private fun place(inspector: Inspector): SectionBuilder? {
        val position = inspector.position ?: return null
        val tile = inspector.tile
        val section = SectionBuilder(Sym.open_with, "Place", if (tile == 1.0) "in tiles" else "in tiles of ${format(tile)}")
        section.add(
            RowUi.Stepper("Across", "X", Color(0xFFD6605C), format(position[0] / tile)),
            RowBinding.PlaceStep("across", position[0], step = tile * 0.5),
        )
        section.add(
            RowUi.Stepper("Up", "Y", Color(0xFF7FB069), format(position[1] / tile)),
            RowBinding.PlaceStep("up", position[1], step = tile * 0.5),
        )
        val turn = SliderScale(-180.0, 180.0)
        section.add(RowUi.Slider("Turn", turn.fraction(inspector.turn), "${inspector.turn.roundToLong()}°"), RowBinding.PlaceSlider("turn", turn, whole = true))
        val size = abs(inspector.scale.getOrElse(1) { 1.0 })
        val sizes = SliderScale(0.25, maxOf(5.0, size))
        section.add(RowUi.Slider("Size", sizes.fraction(size), "${String.format(Locale.ROOT, "%.1f", size)}×"), RowBinding.PlaceSlider("size", sizes, step = 0.05))
        return section
    }

    private fun light(light: Component, tile: Double): SectionBuilder {
        val section = SectionBuilder(Sym.sunny, "Light 2D", "when lit")
        if (light.value("enabled") == false) section.add(RowUi.Toggle("On", false), RowBinding.FieldSwitch(LIGHT, "enabled"))
        slider(section, light, "radius", "Radius", 0.25 * tile, 12.0 * tile, unit = tile, suffix = " tiles", step = 0.05)
        slider(section, light, "intensity", "Brightness", 0.0, 3.0, step = 0.05)
        val color = (light.value("color") as? List<*>)?.mapNotNull { (it as? Number)?.toDouble() }
        if (color != null && color.size == 4) {
            val known = lightColors.indexOfFirst { preset -> preset.indices.all { abs(preset[it] - color[it]) < 0.02 } }
            val choices = if (known >= 0) lightColors else listOf(color.take(3)) + lightColors
            section.add(
                RowUi.Swatches("Color", choices.map { Color(it[0].toFloat().coerceIn(0f, 1f), it[1].toFloat().coerceIn(0f, 1f), it[2].toFloat().coerceIn(0f, 1f)) }, if (known >= 0) known else 0),
                RowBinding.FieldChoice(LIGHT, "color", choices.map { it + color[3] }),
            )
        }
        (light.value("shadows") as? Boolean)?.let { section.add(RowUi.Toggle("Casts shadows", it), RowBinding.FieldSwitch(LIGHT, "shadows")) }
        return section
    }

    private fun sprite(sprite: Component, inspector: Inspector, assetRoot: String): SectionBuilder {
        val section = SectionBuilder(Sym.image, "Sprite")
        val region = inspector.sprite
        val texture = region?.texture ?: (sprite.value("texture") as? String).orEmpty()
        val file = texture.substringAfterLast('/')
        val layer = sprite.number("layer")?.toLong() ?: 0
        val detail = listOfNotNull(region?.region, "layer $layer").joinToString(" · ")
        val image = region?.rect?.takeIf { it.size == 4 }?.let { ProjectImage(assetRoot + region.texture, it) }
        section.add(RowUi.SpriteRef("Sprite", null, 0, file, detail, image), RowBinding.NotWired("Swapping a sprite"))
        section.add(RowUi.Toggle("Mirror", inspector.scale.getOrElse(0) { 1.0 } < 0), RowBinding.PlaceSwitch("mirror"))
        section.add(RowUi.Stepper("Draw layer", "", Forge.TextFaint, layer.toString()), RowBinding.FieldStep(SPRITE, "layer", layer.toDouble(), 1.0, whole = true))
        return section
    }

    private fun collider(collider: Component, tile: Double): SectionBuilder {
        val section = SectionBuilder(Sym.crop_square, "Collider 2D")
        val pieces = (collider.value("pieces") as? List<*>)?.size ?: 0
        val at = if (pieces > 0) "pieces.0." else ""
        val shape = collider.value("${at}shape.shape") as? String ?: return section
        val shapes = listOf("circle", "box")
        if (shape in shapes) {
            val reach = shapeReach(collider, at, shape) ?: 0.5
            section.add(
                RowUi.Segment("Shape", listOf("Circle", "Box"), shapes.indexOf(shape)),
                RowBinding.FieldChoice(COLLIDER, "${at}shape", listOf(mapOf("shape" to "circle", "radius" to reach), mapOf("shape" to "box", "half_extents" to listOf(reach, reach)))),
            )
        }
        (collider.value("${at}sensor") as? Boolean)?.let { section.add(RowUi.Toggle("Sensor (doesn’t block)", it), RowBinding.FieldSwitch(COLLIDER, "${at}sensor")) }
        when (shape) {
            "circle" -> slider(section, collider, "${at}shape.radius", "Touch reach", 0.05 * tile, 3.0 * tile, unit = tile, suffix = " tiles", step = 0.05)
            "box" -> {
                val half = (collider.value("${at}shape.half_extents") as? List<*>)?.mapNotNull { (it as? Number)?.toDouble() }
                if (half != null && half.size == 2) {
                    section.add(RowUi.Stepper("Width", "", Forge.TextFaint, format(half[0] * 2 / tile)), RowBinding.FieldVector(COLLIDER, "${at}shape.half_extents", half, 0, step = tile * 0.25, least = 0.01))
                    section.add(RowUi.Stepper("Height", "", Forge.TextFaint, format(half[1] * 2 / tile)), RowBinding.FieldVector(COLLIDER, "${at}shape.half_extents", half, 1, step = tile * 0.25, least = 0.01))
                }
            }
        }
        if (pieces > 1) section.add(RowUi.Facts("More shapes", listOf("Also" to "${pieces - 1} more, in Details")))
        return section
    }

    private fun shapeReach(collider: Component, at: String, shape: String): Double? = when (shape) {
        "circle" -> collider.number("${at}shape.radius")
        "box" -> (collider.value("${at}shape.half_extents") as? List<*>)?.mapNotNull { (it as? Number)?.toDouble() }?.maxOrNull()
        else -> null
    }

    private fun body(body: Component): SectionBuilder {
        val section = SectionBuilder(Sym.directions_walk, "Body")
        val kind = body.value("kind") as? String
        val kinds = listOf("static", "dynamic", "kinematic_position")
        if (kind != null) {
            val chosen = if (kind.startsWith("kinematic")) 2 else kinds.indexOf(kind)
            if (chosen >= 0) {
                section.add(RowUi.Segment("Moves", listOf("Fixed", "Physics", "Script"), chosen), RowBinding.FieldChoice(BODY, "kind", kinds))
            }
        }
        (body.value("lock_rotation") as? Boolean)?.let { section.add(RowUi.Toggle("Stays upright", it), RowBinding.FieldSwitch(BODY, "lock_rotation")) }
        if (kind == "dynamic") slider(section, body, "gravity_scale", "Gravity", 0.0, 2.0, suffix = "×", step = 0.05)
        return section
    }

    private fun shadow(shadow: Component): SectionBuilder {
        val section = SectionBuilder(Sym.blur_on, "Shadow")
        (shadow.value("cast") as? Boolean)?.let { section.add(RowUi.Toggle("Casts a shadow", it), RowBinding.FieldSwitch(SHADOW, "cast")) }
        slider(section, shadow, "contact_opacity", "Darkness underneath", 0.0, 1.0, step = 0.05)
        return section
    }

    private fun occluder(occluder: Component): SectionBuilder {
        val section = SectionBuilder(Sym.visibility_off, "Blocks light")
        (occluder.value("enabled") as? Boolean)?.let { section.add(RowUi.Toggle("Blocks light", it), RowBinding.FieldSwitch(OCCLUDER, "enabled")) }
        return section
    }

    private fun behaviour(script: Component): SectionBuilder {
        val section = SectionBuilder(Sym.conversion_path, "Behaviour")
        val name = script.value("script") as? String ?: "a script"
        val source = (script.value("source") as? String)?.substringAfterLast('/')
        section.add(RowUi.Behaviour(if (source != null) "Runs $name, from $source." else "Runs $name."))
        return section
    }

    /** A component with no presentation of its own: its numbers, switches and short choices. */
    private fun plain(component: Component): SectionBuilder? {
        val section = SectionBuilder(Sym.extension, component.display, component.type)
        component.values.forEach { (name, value) ->
            val meaning = component.meanings[name]
            when {
                value is Boolean -> section.add(RowUi.Toggle(name, value), RowBinding.FieldSwitch(component.type, name))
                value is Number && meaning?.kind == "range" && meaning.min != null && meaning.max != null ->
                    slider(section, component, name, name, meaning.min, meaning.max, step = (meaning.max - meaning.min) / 100)
                value is Number -> {
                    val whole = value is Int || value is Long
                    section.add(
                        RowUi.Stepper(name, "", Forge.TextFaint, format(value.toDouble())),
                        RowBinding.FieldStep(component.type, name, value.toDouble(), if (whole) 1.0 else 0.1, whole, meaning?.min, meaning?.max),
                    )
                }
                value is String && meaning?.kind == "choice" && meaning.options.size in 2..3 ->
                    section.add(RowUi.Segment(name, meaning.options.map(::friendly), meaning.options.indexOf(value)), RowBinding.FieldChoice(component.type, name, meaning.options))
            }
        }
        return section.takeIf { it.rows.isNotEmpty() }
    }

    private fun details(inspector: Inspector, open: Boolean): SectionBuilder {
        val section = SectionBuilder(Sym.info, "Details")
        section.add(RowUi.More(if (open) "Hide details" else "Show all details", open), RowBinding.ToggleDetails)
        if (open) {
            val facts = buildList {
                inspector.id?.let { add("Stable ID" to it) }
                inspector.position?.let { add("Position" to it.joinToString(", ") { value -> format(value) }) }
                add("Scale" to inspector.scale.joinToString(", ") { format(it) })
                inspector.sprite?.texture?.let { add("Texture" to it) }
                inspector.components.forEach { component ->
                    add(component.display to component.type)
                }
            }
            section.add(RowUi.Facts("Details", facts))
        }
        return section
    }

    /** A slider row over [min]–[max] for the number at [path], in [unit]s, if the component has it. */
    private fun slider(
        section: SectionBuilder,
        component: Component,
        path: String,
        label: String,
        min: Double,
        max: Double,
        unit: Double = 1.0,
        suffix: String = "",
        step: Double = 0.0,
    ) {
        val value = component.number(path) ?: return
        val meaning = component.meanings[path] ?: component.meanings[path.replace(Regex("\\.\\d+\\."), "[].")]
        // The engine's bounds win where they are narrower; a value already
        // past the design's range widens it rather than being clamped.
        val low = maxOf(min, meaning?.min ?: min).coerceAtMost(value)
        val high = minOf(max, meaning?.max ?: max).coerceAtLeast(value)
        val scale = SliderScale(low, high)
        section.add(RowUi.Slider(label, scale.fraction(value), format(value / unit) + suffix), RowBinding.FieldSlider(component.type, path, scale, step))
    }

    private fun friendly(spelling: String) = spelling.replace('_', ' ').replaceFirstChar { it.uppercase() }

    fun format(value: Double): String =
        if (value == Math.rint(value)) value.toLong().toString() else String.format(Locale.ROOT, "%.2f", value).trimEnd('0').trimEnd('.')
}

/** A slider's range: where along it a value sits, and the value at a fraction of it. */
data class SliderScale(val min: Double, val max: Double) {
    fun fraction(value: Double): Float = if (max <= min) 0f else ((value - min) / (max - min)).toFloat().coerceIn(0f, 1f)

    fun value(fraction: Float): Double = min + (max - min) * fraction.coerceIn(0f, 1f)
}

/** One native edit: a host command and its arguments. */
data class NativeEdit(val command: String, val arguments: List<Any?>)

/**
 * What an inspector row edits. [edit] turns a change to the row into the
 * native command that makes it; `merging` is set while a slider is dragged,
 * so a drag is one undo step.
 */
sealed interface RowBinding {
    fun edit(change: RowChange): NativeEdit?

    data class PlaceStep(val what: String, val current: Double, val step: Double) : RowBinding {
        override fun edit(change: RowChange) = when (change) {
            RowChange.Minus -> NativeEdit("setPlace", listOf(what, round(current - step), false))
            RowChange.Plus -> NativeEdit("setPlace", listOf(what, round(current + step), false))
            else -> null
        }
    }

    data class PlaceSlider(val what: String, val scale: SliderScale, val step: Double = 0.0, val whole: Boolean = false) : RowBinding {
        override fun edit(change: RowChange) = (change as? RowChange.To)?.let {
            var value = snap(scale.value(it.fraction), step)
            if (whole) value = Math.rint(value)
            NativeEdit("setPlace", listOf(what, value, true))
        }
    }

    data class PlaceSwitch(val what: String) : RowBinding {
        override fun edit(change: RowChange) = (change as? RowChange.Switch)?.let { NativeEdit("setPlace", listOf(what, it.on, false)) }
    }

    data class FieldStep(
        val type: String,
        val path: String,
        val current: Double,
        val step: Double,
        val whole: Boolean,
        val min: Double? = null,
        val max: Double? = null,
    ) : RowBinding {
        override fun edit(change: RowChange): NativeEdit? {
            val direction = when (change) {
                RowChange.Minus -> -1
                RowChange.Plus -> 1
                else -> return null
            }
            var next = current + direction * step
            min?.let { next = maxOf(it, next) }
            max?.let { next = minOf(it, next) }
            val value: Any = if (whole) Math.round(next) else round(next)
            return NativeEdit("setField", listOf(type, path, value, false))
        }
    }

    data class FieldSlider(val type: String, val path: String, val scale: SliderScale, val step: Double = 0.0) : RowBinding {
        override fun edit(change: RowChange) = (change as? RowChange.To)?.let {
            NativeEdit("setField", listOf(type, path, snap(scale.value(it.fraction), step), true))
        }
    }

    /** One number of a list field, such as a box's half width: the list is written whole. */
    data class FieldVector(val type: String, val path: String, val current: List<Double>, val index: Int, val step: Double, val least: Double) : RowBinding {
        override fun edit(change: RowChange): NativeEdit? {
            val direction = when (change) {
                RowChange.Minus -> -1
                RowChange.Plus -> 1
                else -> return null
            }
            // Steps are of the whole width, which is twice the half stored.
            val next = current.toMutableList().also { it[index] = round(maxOf(least, it[index] + direction * step / 2)) }
            return NativeEdit("setField", listOf(type, path, next, false))
        }
    }

    data class FieldSwitch(val type: String, val path: String) : RowBinding {
        override fun edit(change: RowChange) = (change as? RowChange.Switch)?.let { NativeEdit("setField", listOf(type, path, it.on, false)) }
    }

    /** A segment or swatch: the value each choice writes. */
    data class FieldChoice(val type: String, val path: String, val values: List<Any?>) : RowBinding {
        override fun edit(change: RowChange) = (change as? RowChange.Choose)?.let { chosen ->
            values.getOrNull(chosen.index)?.let { NativeEdit("setField", listOf(type, path, it, false)) }
        }
    }

    data class NotWired(val what: String) : RowBinding {
        override fun edit(change: RowChange): NativeEdit? = null
    }

    data object ToggleDetails : RowBinding {
        override fun edit(change: RowChange): NativeEdit? = null
    }

    companion object {
        fun round(value: Double) = Math.round(value * 1000.0) / 1000.0

        fun snap(value: Double, step: Double) = if (step > 0) round(Math.round(value / step) * step) else round(value)
    }
}
