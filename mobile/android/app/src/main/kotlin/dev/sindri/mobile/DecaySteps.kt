package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.FlowRowScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** One line of a handler as the Steps view lists it: a statement, or the heading of a branch. */
sealed interface StepRow {
    val depth: Int

    data class Step(val node: DecayNode, override val depth: Int) : StepRow

    data class Branch(val label: String, override val depth: Int) : StepRow
}

/**
 * The statements of [handler] in [graph], in the order they run, as rows:
 * each branch or loop body indented under its statement, an If's False
 * headed "Otherwise". The same reading as the nodes, so every edit applies.
 */
fun steps(graph: DecayGraph, handler: DecayNode): List<StepRow> {
    val byKey = graph.nodes.associateBy { it.key }
    val seen = HashSet<String>()
    val rows = mutableListOf<StepRow>()
    fun next(from: String, out: String) =
        graph.wires.firstOrNull { it.flow && it.from == from && it.out == out }?.let { byKey[it.to] }
    fun chain(start: DecayNode?, depth: Int) {
        var node = start
        while (node != null && seen.add(node.key)) {
            rows += StepRow.Step(node, depth)
            val branches = node.outputs.filter { it.isNotEmpty() && it != "Next" }
            branches.forEachIndexed { index, branch ->
                val first = next(node!!.key, branch)
                if (index > 0 && first != null) rows += StepRow.Branch(if (branch == "False") "Otherwise" else branch, depth)
                chain(first, depth + 1)
            }
            node = next(node.key, if (branches.isEmpty()) node.outputs.firstOrNull() ?: "" else "Next")
        }
    }
    chain(next(handler.key, handler.outputs.firstOrNull() ?: ""), 0)
    return rows
}

/**
 * A script as steps: one handler at a time, each statement a sentence with
 * its values as chips, read top to bottom like the code it is. Tap a value
 * to change it, a step for its menu.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun DecaySteps(
    script: DecayScript,
    onEdit: (DecaySlot, Any) -> Unit,
    onChange: (kind: String, span: Pair<Int, Int>, value: Any?) -> Unit,
) {
    val graph = script.graph
    val handlers = graph.nodes.filter { it.kind in listOf("event", "function", "values") }
    var shown by remember { mutableStateOf<String?>(null) }
    val current = handlers.firstOrNull { it.key == shown }
        ?: handlers.firstOrNull { it.kind == "event" } ?: handlers.firstOrNull()
    val editable = script.problems.isEmpty()
    var editing by remember { mutableStateOf<DecaySlot?>(null) }
    var menu by remember { mutableStateOf<DecayNode?>(null) }
    var adding by remember { mutableStateOf<((String) -> Unit)?>(null) }
    val tap: ((DecaySlot) -> Unit)? = if (!editable) null else { slot ->
        if (slot is DecaySlot.Bool) onEdit(slot, !slot.value) else editing = slot
    }

    Column(Modifier.fillMaxSize().background(Forge.Well)) {
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(8.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            handlers.forEach { handler ->
                val on = handler.key == current?.key
                Chip(if (handler.kind == "values") "Values" else handler.title, on) { shown = handler.key }
            }
        }
        if (!editable) {
            Text(
                "Fix this script's errors as code to edit it here:\n" + script.problems.joinToString("\n"),
                style = ForgeType.text(12.sp, color = Forge.Danger),
                modifier = Modifier.padding(12.dp),
            )
        }
        Column(
            Modifier.fillMaxWidth().weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 12.dp, vertical = 4.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            current?.note?.let { Text(it, style = ForgeType.text(12.sp, color = Forge.TextMuted)) }
            when {
                current == null -> Unit
                current.kind == "values" -> current.inputs.forEach { pin ->
                    Card(0, null) { Sentence(pin.name, listOf(pin), graph, tap) }
                }
                else -> {
                    val rows = steps(graph, current)
                    rows.forEach { row ->
                        when (row) {
                            is StepRow.Branch -> Text(
                                row.label,
                                style = ForgeType.text(12.sp, FontWeight.SemiBold, Forge.TextMuted),
                                modifier = Modifier.padding(start = (row.depth * INDENT).dp, top = 4.dp),
                            )
                            is StepRow.Step -> Card(row.depth, if (editable) ({ menu = row.node }) else null, row.node.title) {
                                row.node.note?.let { Text(it, style = ForgeType.text(11.sp, color = Forge.TextFaint)) }
                                Sentence(row.node.title, row.node.inputs, graph, tap, row.node)
                            }
                        }
                    }
                    if (editable) {
                        val last = rows.lastOrNull { it is StepRow.Step && it.depth == 0 } as? StepRow.Step
                        Text(
                            "+  Add a step",
                            style = ForgeType.text(14.sp, FontWeight.SemiBold, Forge.MetalBright),
                            modifier = Modifier
                                .padding(top = 6.dp)
                                .clip(RoundedCornerShape(12.dp))
                                .border(1.dp, Forge.MetalLine, RoundedCornerShape(12.dp))
                                .clickable {
                                    adding = { id ->
                                        if (last != null) {
                                            onChange("insertAfter", last.node.span, id)
                                        } else {
                                            onChange("insertInside", current.span, mapOf("template" to id, "branch" to 0))
                                        }
                                    }
                                }
                                .padding(horizontal = 14.dp, vertical = 12.dp),
                        )
                    }
                }
            }
        }
    }
    editing?.let { slot ->
        SlotDialog(
            slot,
            fields = script.fieldsFor(slot),
            onField = { field ->
                editing = null
                ((slot as? DecaySlot.Number)?.span ?: (slot as? DecaySlot.Text)?.span)?.let { onChange("useField", it, field) }
            },
        ) { value -> editing = null; value?.let { onEdit(slot, it) } }
    }
    menu?.let { node -> NodeMenu(node, script.palette, onDone = { menu = null }, onChange = onChange) }
    adding?.let { add ->
        AlertDialog(
            onDismissRequest = { adding = null },
            containerColor = Forge.Popover,
            title = { Text("Add a step", style = ForgeType.text(16.sp, FontWeight.SemiBold)) },
            text = {
                Column(Modifier.heightIn(max = 420.dp).verticalScroll(rememberScrollState())) {
                    script.palette.forEach { (id, label) -> MenuRow(label) { add(id); adding = null } }
                }
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { adding = null }) { Text("Cancel", color = Forge.TextMuted) } },
        )
    }
}

private const val INDENT = 20

@Composable
private fun Chip(label: String, on: Boolean, onClick: () -> Unit) = Text(
    label,
    style = ForgeType.text(12.sp, FontWeight.SemiBold, if (on) Forge.MetalBright else Forge.TextMuted),
    maxLines = 1,
    modifier = Modifier
        .clip(RoundedCornerShape(10.dp))
        .background(if (on) Forge.MetalDeep else Forge.Raised)
        .border(1.dp, if (on) Forge.MetalLine else Forge.Line, RoundedCornerShape(10.dp))
        .clickable(onClick = onClick)
        .padding(horizontal = 10.dp, vertical = 7.dp),
)

@Composable
private fun Card(depth: Int, onMenu: (() -> Unit)?, title: String? = null, content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(12.dp)
    Column(
        Modifier
            .padding(start = (depth * INDENT).dp)
            .fillMaxWidth()
            .clip(shape)
            .background(Forge.Raised)
            .border(1.dp, Forge.Line, shape)
            .let { if (onMenu != null) it.clickable(onClick = onMenu).semantics { contentDescription = "$title, tap for its menu" } else it }
            .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) { content() }
}

/**
 * A statement as a sentence: its [title], then each pin's value, a wired
 * value written in place as its own title and values.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Sentence(title: String, pins: List<DecayPin>, graph: DecayGraph, tap: ((DecaySlot) -> Unit)?, node: DecayNode? = null) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(title, style = ForgeType.text(14.sp, FontWeight.SemiBold), modifier = Modifier.align(Alignment.CenterVertically))
        Values(pins, node, graph, tap)
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun FlowRowScope.Values(pins: List<DecayPin>, node: DecayNode?, graph: DecayGraph, tap: ((DecaySlot) -> Unit)?) {
    pins.forEach { pin ->
        if (pin.name.isNotEmpty() && pin.name.any { !it.isDigit() }) {
            Text(pin.name, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.align(Alignment.CenterVertically))
        }
        val value = pin.value
        if (value != null) {
            SlotChip(value, tap?.takeIf { value.editable }?.let { { it(value) } }, Modifier.align(Alignment.CenterVertically))
        } else if (node != null) {
            val wire = graph.wires.firstOrNull { !it.flow && it.to == node.key && it.into == pin.name }
            graph.nodes.firstOrNull { it.key == wire?.from }?.let { from ->
                Text(from.title, style = ForgeType.text(13.sp, FontWeight.Medium, Forge.Info), modifier = Modifier.align(Alignment.CenterVertically))
                Values(from.inputs, from, graph, tap)
            }
        }
    }
}
