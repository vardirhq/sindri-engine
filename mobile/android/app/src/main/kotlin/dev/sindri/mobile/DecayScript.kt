package dev.sindri.mobile

import org.json.JSONArray
import org.json.JSONObject

/**
 * A `.decay` script as blocks: what `sindri_authoring::visual_decay` reads it
 * as. The script file is the only thing stored; this is how the phone shows it,
 * and every edit made here is a change to one slot's text in that file.
 */
data class DecayScript(
    /** As the script component names it, such as `scripts/wick.decay`. */
    val name: String,
    val source: String,
    val containers: List<DecayContainer>,
    /** What the parser could not read; with any, blocks are not edited. */
    val problems: List<String> = emptyList(),
    /** The same reading as a node graph, placed, for the canvas. */
    val graph: DecayGraph = DecayGraph(emptyList(), emptyList()),
    /** The statements a node menu can add: an id the host knows, and its words. */
    val palette: List<Pair<String, String>> = emptyList(),
) {
    companion object {
        /** The native host's `script()` answer, or null for `null`. */
        fun read(json: String): DecayScript? {
            if (json.isBlank() || json == "null") return null
            val root = JSONObject(json)
            val blocks = root.getJSONObject("blocks")
            return DecayScript(
                name = root.optString("name"),
                source = root.optString("source"),
                containers = blocks.optJSONArray("containers").objects().map(::container),
                problems = blocks.optJSONArray("problems").objects().map { "Line ${it.optInt("line")}: ${it.optString("message")}" },
                graph = root.optJSONObject("graph")?.let(::graph) ?: DecayGraph(emptyList(), emptyList()),
                palette = root.optJSONArray("palette").objects().map { it.optString("id") to it.optString("label") },
            )
        }

        private fun graph(json: JSONObject) = DecayGraph(
            nodes = json.optJSONArray("nodes").objects().map { node ->
                val at = node.optJSONArray("at")
                DecayNode(
                    key = node.optString("key"),
                    kind = node.optString("kind"),
                    title = node.optString("title"),
                    note = node.optStringOrNull("note"),
                    inputs = node.optJSONArray("inputs").objects().map { pin -> DecayPin(pin.optString("name"), pin.optJSONObject("value")?.let(::slot)) },
                    outputs = node.optJSONArray("outputs").strings(),
                    x = at?.optDouble(0)?.toFloat() ?: 0f,
                    y = at?.optDouble(1)?.toFloat() ?: 0f,
                    span = node.optJSONArray("span")?.let { it.optInt(0) to it.optInt(1) } ?: (0 to 0),
                )
            },
            wires = json.optJSONArray("wires").objects().map { wire ->
                DecayWire(wire.optString("from"), wire.optString("out"), wire.optString("to"), wire.optString("into"), wire.optBoolean("flow"))
            },
            flow = json.optString("flow", "right"),
        )

        private fun container(json: JSONObject) = DecayContainer(
            kind = json.optString("kind"),
            name = json.optString("name"),
            fields = json.optJSONArray("fields").objects().map { field ->
                DecayField(
                    name = field.optString("name"),
                    exported = field.optBoolean("exported"),
                    mutable = field.optBoolean("mutable"),
                    type = field.optStringOrNull("type"),
                    value = field.optJSONObject("value")?.let(::slot),
                    note = field.optStringOrNull("note"),
                )
            },
            handlers = json.optJSONArray("handlers").objects().map { handler ->
                DecayHandler(
                    name = handler.optString("name"),
                    label = handler.optString("label"),
                    event = handler.optString("role") == "event",
                    params = handler.optJSONArray("params").strings(),
                    body = blocks(handler.optJSONArray("body")),
                    note = handler.optStringOrNull("note"),
                )
            },
        )

        private fun blocks(json: JSONArray?): List<DecayBlock> = json.objects().map(::block)

        private fun block(json: JSONObject): DecayBlock = when (json.optString("kind")) {
            "set" -> DecayBlock.Set(slot(json.getJSONObject("target")), slot(json.getJSONObject("value")))
            "change" -> DecayBlock.Change(slot(json.getJSONObject("target")), json.optString("op"), slot(json.getJSONObject("value")))
            "let" -> DecayBlock.Let(json.optString("name"), json.optBoolean("mutable"), json.optJSONObject("value")?.let(::slot))
            "do" -> DecayBlock.Do(slot(json.getJSONObject("call")))
            "if" -> DecayBlock.If(slot(json.getJSONObject("condition")), blocks(json.optJSONArray("then")), json.optJSONArray("otherwise")?.let(::blocks))
            "while" -> DecayBlock.While(slot(json.getJSONObject("condition")), blocks(json.optJSONArray("body")))
            "for" -> DecayBlock.For(json.optString("name"), slot(json.getJSONObject("items")), blocks(json.optJSONArray("body")))
            "match" -> DecayBlock.Match(
                slot(json.getJSONObject("subject")),
                json.optJSONArray("arms").objects().map { arm -> DecayArm(arm.optJSONArray("patterns").strings(), blocks(arm.optJSONArray("body"))) },
            )
            "return" -> DecayBlock.Return(json.optJSONObject("value")?.let(::slot))
            "break" -> DecayBlock.Break
            "continue" -> DecayBlock.Continue
            "comment" -> DecayBlock.Comment(json.optString("text"))
            else -> DecayBlock.Code(json.optString("source"))
        }

        private fun slot(json: JSONObject): DecaySlot {
            val span = json.optJSONArray("span").let { if (it == null) 0 to 0 else it.getInt(0) to it.getInt(1) }
            return when (json.optString("kind")) {
                "number" -> DecaySlot.Number(json.optDouble("value"), span)
                "text" -> DecaySlot.Text(json.optString("value"), span)
                "bool" -> DecaySlot.Bool(json.optBoolean("value"), span)
                "null" -> DecaySlot.Null
                "name" -> DecaySlot.Name(json.optString("path"))
                "call" -> DecaySlot.Call(json.optString("callee"), json.optJSONArray("args").objects().map(::slot))
                "op" -> DecaySlot.Op(json.optString("op"), slot(json.getJSONObject("left")), slot(json.getJSONObject("right")))
                "unary" -> DecaySlot.Unary(json.optString("op"), slot(json.getJSONObject("operand")))
                else -> DecaySlot.Code(json.optString("source"))
            }
        }

        private fun JSONArray?.objects(): List<JSONObject> = if (this == null) emptyList() else List(length()) { getJSONObject(it) }

        private fun JSONArray?.strings(): List<String> = if (this == null) emptyList() else List(length()) { getString(it) }

        private fun JSONObject.optStringOrNull(key: String): String? = if (!has(key) || isNull(key)) null else optString(key)
    }
}

/**
 * A script as Blueprint-style nodes: event, statement and value nodes, and the
 * wires between them. [flow] is which way execution runs: `down`, as the
 * phone lays it out, or `right`.
 */
data class DecayGraph(val nodes: List<DecayNode>, val wires: List<DecayWire>, val flow: String = "right")

/**
 * One node, at [x], [y] in points on the canvas. [kind] is `event`,
 * `function`, `values`, `statement` or `value`. A statement takes execution
 * in; a value node gives one result out.
 */
data class DecayNode(
    val key: String,
    val kind: String,
    val title: String,
    val note: String?,
    val inputs: List<DecayPin>,
    val outputs: List<String>,
    val x: Float,
    val y: Float,
    /** Where it is in the script: what a change to it is made at. */
    val span: Pair<Int, Int> = 0 to 0,
) {
    val takesFlow get() = kind == "statement"
    val givesValue get() = kind == "value"
}

/** An input pin: what it holds on the pin, or null where a wire feeds it. */
data class DecayPin(val name: String, val value: DecaySlot?)

/** Execution from [from]'s [out] to [to], or a value from [from] into [to]'s [into] pin. */
data class DecayWire(val from: String, val out: String, val to: String, val into: String, val flow: Boolean)

data class DecayContainer(val kind: String, val name: String, val fields: List<DecayField>, val handlers: List<DecayHandler>)

/** A value the script keeps; an `@export` one is tuned per object. */
data class DecayField(
    val name: String,
    val exported: Boolean,
    val mutable: Boolean,
    val type: String?,
    val value: DecaySlot?,
    val note: String?,
)

/** A "When …" block: an engine event, an event handler or the script's own function. */
data class DecayHandler(
    val name: String,
    val label: String,
    val event: Boolean,
    val params: List<String>,
    val body: List<DecayBlock>,
    val note: String?,
)

/** One statement, as a block. */
sealed interface DecayBlock {
    data class Set(val target: DecaySlot, val value: DecaySlot) : DecayBlock
    data class Change(val target: DecaySlot, val op: String, val value: DecaySlot) : DecayBlock
    data class Let(val name: String, val mutable: Boolean, val value: DecaySlot?) : DecayBlock
    data class Do(val call: DecaySlot) : DecayBlock
    data class If(val condition: DecaySlot, val then: List<DecayBlock>, val otherwise: List<DecayBlock>?) : DecayBlock
    data class While(val condition: DecaySlot, val body: List<DecayBlock>) : DecayBlock
    data class For(val name: String, val items: DecaySlot, val body: List<DecayBlock>) : DecayBlock
    data class Match(val subject: DecaySlot, val arms: List<DecayArm>) : DecayBlock
    data class Return(val value: DecaySlot?) : DecayBlock
    data object Break : DecayBlock
    data object Continue : DecayBlock
    data class Comment(val text: String) : DecayBlock
    /** A statement with no block of its own yet, as its exact source. */
    data class Code(val source: String) : DecayBlock
}

data class DecayArm(val patterns: List<String>, val body: List<DecayBlock>)

/** An expression in a block's slot. Numbers, text and switches edit in place; their [span] is where. */
sealed interface DecaySlot {
    data class Number(val value: Double, val span: Pair<Int, Int>) : DecaySlot
    data class Text(val value: String, val span: Pair<Int, Int>) : DecaySlot
    data class Bool(val value: Boolean, val span: Pair<Int, Int>) : DecaySlot
    data object Null : DecaySlot
    data class Name(val path: String) : DecaySlot
    data class Call(val callee: String, val args: List<DecaySlot>) : DecaySlot
    data class Op(val op: String, val left: DecaySlot, val right: DecaySlot) : DecaySlot
    data class Unary(val op: String, val operand: DecaySlot) : DecaySlot
    data class Code(val source: String) : DecaySlot
}
