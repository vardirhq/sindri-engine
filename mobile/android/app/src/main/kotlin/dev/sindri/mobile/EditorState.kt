package dev.sindri.mobile

import org.json.JSONArray
import org.json.JSONObject

/** One editable field of a component: a number or a switch. */
data class Field(val name: String, val value: Any) {
    val isSwitch get() = value is Boolean

    /** Whether the value was stored as a whole number, which it stays. */
    val isWhole get() = value is Int || value is Long

    /** Whole numbers step by one; anything with a fraction by a tenth. */
    val step: Double get() = if (isWhole) 1.0 else 0.1
}

/** What the engine says a field means: a bounded number, one of some spellings, a colour … */
data class Meaning(val kind: String, val min: Double? = null, val max: Double? = null, val options: List<String> = emptyList())

data class Component(
    val type: String,
    val display: String,
    /** Its top-level numbers and switches, for a component with no presentation of its own. */
    val fields: List<Field>,
    /** Every field at its effective value, defaults included: maps, lists, numbers, text, switches. */
    val values: Map<String, Any?> = emptyMap(),
    /** Field path (`radius`, `pieces[].sensor`) to what the engine says it means. */
    val meanings: Map<String, Meaning> = emptyMap(),
) {
    /** The value at a dotted path (`shape.radius`, `pieces.0.sensor`), or null. */
    fun value(path: String): Any? = path.split('.').fold(values as Any?) { at, step ->
        when (at) {
            is Map<*, *> -> at[step]
            is List<*> -> step.toIntOrNull()?.let { at.getOrNull(it) }
            else -> null
        }
    }

    fun number(path: String): Double? = (value(path) as? Number)?.toDouble()
}

/** A sprite's picture: its texture, the named part, and where that part is in the image. */
data class SpriteRegion(val texture: String, val region: String?, val rect: List<Float>?)

data class Inspector(
    val name: String,
    val position: List<Double>?,
    val components: List<Component>,
    val id: String? = null,
    /** Degrees clockwise as the canvas shows it. */
    val turn: Double = 0.0,
    val scale: List<Double> = listOf(1.0, 1.0, 1.0),
    /** World units across one of the scene's tiles. */
    val tile: Double = 1.0,
    val sprite: SpriteRegion? = null,
) {
    fun component(type: String): Component? = components.firstOrNull { it.type == type }
}

/**
 * A game running from the editor: its clock, whether it is paused, and the
 * stick (where the finger landed and where its knob is, and how far it
 * reaches), in surface pixels.
 */
data class PlayState(
    val seconds: Double,
    val paused: Boolean,
    val anchor: Pair<Float, Float>? = null,
    val knob: Pair<Float, Float>? = null,
    val reach: Float = 0f,
)

/** What the native host reports through `state()`, which [EditorController] turns into the design's [EditorUi]. */
data class EditorState(
    val status: String = "",
    val inspector: Inspector? = null,
    /** Where the selection's origin is, in surface pixels. */
    val origin: Pair<Float, Float>? = null,
    /** The box the selection covers: left, top, right, bottom, in surface pixels. */
    val bounds: List<Float>? = null,
    val undo: String? = null,
    val redo: String? = null,
    /** Whether the scene holds edits its file does not. */
    val unsaved: Boolean = false,
    /** Whether the canvas shows the scene's 2D lighting. */
    val lighting: Boolean = false,
    /** The game running from the editor, or null in the editor. */
    val play: PlayState? = null,
    /** Why the last game stopped by itself, until the app clears it. */
    val playError: String? = null,
) {
    companion object {
        fun read(json: String): EditorState {
            val root = JSONObject(json)
            return EditorState(
                status = root.optString("status"),
                inspector = root.optJSONObject("inspector")?.let(::inspector),
                origin = root.optJSONArray("origin")?.let {
                    it.getDouble(0).toFloat() to it.getDouble(1).toFloat()
                },
                bounds = root.optJSONArray("bounds")?.doubles()?.map { it.toFloat() },
                undo = root.optStringOrNull("undo"),
                redo = root.optStringOrNull("redo"),
                unsaved = root.optBoolean("unsaved"),
                lighting = root.optBoolean("lighting"),
                play = root.optJSONObject("play")?.let { play ->
                    val stick = play.optJSONObject("stick")
                    fun point(name: String) = stick?.optJSONArray(name)?.let { it.getDouble(0).toFloat() to it.getDouble(1).toFloat() }
                    PlayState(
                        seconds = play.optDouble("seconds", 0.0),
                        paused = play.optBoolean("paused"),
                        anchor = point("anchor"),
                        knob = point("knob"),
                        reach = play.optDouble("reach", 0.0).toFloat(),
                    )
                },
                playError = root.optStringOrNull("playError"),
            )
        }

        private fun inspector(json: JSONObject) = Inspector(
            name = json.optString("name"),
            position = json.optJSONArray("position")?.doubles(),
            components = json.optJSONArray("components")?.objects()?.map { component ->
                Component(
                    type = component.getString("type"),
                    display = component.optString("display", component.getString("type")),
                    fields = component.optJSONArray("fields")?.objects()?.map { field ->
                        Field(field.getString("name"), field.get("value"))
                    }.orEmpty(),
                    values = component.optJSONObject("values")?.let(::fields).orEmpty(),
                    meanings = component.optJSONObject("meanings")?.let { meanings ->
                        meanings.keys().asSequence().associateWith { path ->
                            val meaning = meanings.getJSONObject(path)
                            Meaning(
                                kind = meaning.optString("kind"),
                                min = meaning.optDoubleOrNull("min"),
                                max = meaning.optDoubleOrNull("max"),
                                options = meaning.optJSONArray("options")?.let { options -> List(options.length()) { options.getString(it) } }.orEmpty(),
                            )
                        }
                    }.orEmpty(),
                )
            }.orEmpty(),
            id = json.optStringOrNull("id"),
            turn = json.optDouble("turn", 0.0).takeUnless { it.isNaN() } ?: 0.0,
            scale = json.optJSONArray("scale")?.doubles() ?: listOf(1.0, 1.0, 1.0),
            tile = json.optDouble("tile", 1.0).takeUnless { it.isNaN() || it <= 0.0 } ?: 1.0,
            sprite = json.optJSONObject("sprite")?.let { sprite ->
                SpriteRegion(
                    texture = sprite.optString("texture"),
                    region = sprite.optStringOrNull("region"),
                    rect = sprite.optJSONArray("rect")?.doubles()?.map { it.toFloat() },
                )
            },
        )

        private fun fields(json: JSONObject): Map<String, Any?> = json.keys().asSequence().associateWith { plain(json.get(it)) }

        /** JSON as plain Kotlin: maps, lists, numbers, text, switches and nulls. */
        private fun plain(value: Any?): Any? = when (value) {
            is JSONObject -> fields(value)
            is JSONArray -> List(value.length()) { plain(value.get(it)) }
            JSONObject.NULL -> null
            else -> value
        }

        private fun JSONObject.optDoubleOrNull(key: String): Double? =
            if (!has(key) || isNull(key)) null else optDouble(key)

        private fun JSONObject.optStringOrNull(key: String): String? =
            if (!has(key) || isNull(key)) null else optString(key)

        private fun JSONArray.doubles() = List(length()) { getDouble(it) }

        private fun JSONArray.objects() = List(length()) { getJSONObject(it) }
    }
}
