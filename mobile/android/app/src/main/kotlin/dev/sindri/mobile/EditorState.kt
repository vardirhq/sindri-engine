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

data class Component(val type: String, val display: String, val fields: List<Field>)

data class Inspector(
    val name: String,
    val position: List<Double>?,
    val components: List<Component>,
)

/** What the overlay draws, read from the native host's `state()`. */
data class EditorState(
    val status: String = "",
    val inspector: Inspector? = null,
    /** Where the selection's origin is, in surface pixels. */
    val marker: Pair<Float, Float>? = null,
    val undo: String? = null,
    val redo: String? = null,
    /** Whether the scene holds edits its file does not. */
    val unsaved: Boolean = false,
) {
    companion object {
        fun read(json: String): EditorState {
            val root = JSONObject(json)
            return EditorState(
                status = root.optString("status"),
                inspector = root.optJSONObject("inspector")?.let(::inspector),
                marker = root.optJSONArray("marker")?.let {
                    it.getDouble(0).toFloat() to it.getDouble(1).toFloat()
                },
                undo = root.optStringOrNull("undo"),
                redo = root.optStringOrNull("redo"),
                unsaved = root.optBoolean("unsaved"),
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
                )
            }.orEmpty(),
        )

        private fun JSONObject.optStringOrNull(key: String): String? =
            if (isNull(key)) null else optString(key)

        private fun JSONArray.doubles() = List(length()) { getDouble(it) }

        private fun JSONArray.objects() = List(length()) { getJSONObject(it) }
    }
}
