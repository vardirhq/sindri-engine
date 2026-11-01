package dev.sindri.mobile

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.clickable
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Decay, but visual, as nodes: Blueprint-style, on a canvas that pans and
 * pinches. Each handler is an event node, its statements a chain joined by
 * execution wires, and each call or operator a value node wired into the pin
 * it feeds. Numbers, text and switches sit on their pins and are tapped to
 * change the script there; a node is dragged by its header, and where it is
 * left is kept beside the script, never in it.
 *
 * Node measures are the shared layout's (`visual_decay::layout`), so wires meet
 * pins without measuring anything.
 */
@Composable
fun DecayCanvas(
    script: DecayScript,
    onEdit: (DecaySlot, Any) -> Unit,
    onMove: (key: String, x: Float, y: Float) -> Unit,
    onChange: (kind: String, span: Pair<Int, Int>, value: Any?) -> Unit = { _, _, _ -> },
) {
    // One handler at a time, as an event graph is in Blueprints: its tab,
    // then its nodes. The Values node has a tab of its own.
    val handlers = script.graph.nodes.filter { it.kind in listOf("event", "function", "values") }
    var shown by remember { mutableStateOf<String?>(null) }
    val current = shown?.takeIf { key -> handlers.any { it.key == key } }
        ?: (handlers.firstOrNull { it.kind == "event" } ?: handlers.firstOrNull())?.key
    val graph = remember(script.graph, current) { only(script.graph, current) }
    val editable = script.problems.isEmpty()
    var pan by remember { mutableStateOf(Offset(24f, 24f)) }
    var zoom by remember { mutableStateOf(0.8f) }
    // A handler's first view shows all of it, unless that would make it too small to read.
    var framed by remember(current) { mutableStateOf(false) }
    // Where nodes are while a finger drags them, ahead of the host.
    val dragged = remember(graph) { mutableStateMapOf<String, Offset>() }
    var editing by remember { mutableStateOf<DecaySlot?>(null) }
    var menu by remember { mutableStateOf<DecayNode?>(null) }
    val density = LocalDensity.current.density
    val place = { node: DecayNode -> dragged[node.key] ?: Offset(node.x, node.y) }
    val tap: (DecaySlot) -> Unit = { slot ->
        when (slot) {
            is DecaySlot.Bool -> onEdit(slot, !slot.value)
            else -> editing = slot
        }
    }

    BoxWithConstraints(
        Modifier
            .fillMaxSize()
            .clipToBounds()
            .background(Forge.Well)
            .pointerInput(Unit) {
                detectTransformGestures { centroid, move, scale, _ ->
                    val next = (zoom * scale).coerceIn(0.3f, 2f)
                    // Keep the point under the fingers where it is.
                    pan = centroid - (centroid - pan) * (next / zoom) + move
                    zoom = next
                }
            },
    ) {
        val (wide, tall) = maxWidth.value to maxHeight.value
        if (!framed) {
            framed = true
            val placed = graph.nodes.map { Offset(it.x, it.y) to it }
            val (fitPan, fitZoom) = fit(placed, wide, tall)
            zoom = fitZoom.coerceAtLeast(READABLE)
            // Too big to read whole: start at its top left, where the handler begins.
            pan = if (fitZoom >= READABLE || placed.isEmpty()) {
                fitPan * density
            } else {
                Offset(16f - placed.minOf { it.first.x } * READABLE, TABS - nodesTop(placed) * READABLE) * density
            }
        }
        Box(
            Modifier
                .wrapContentSize(Alignment.TopStart, unbounded = true)
                .graphicsLayer {
                    translationX = pan.x
                    translationY = pan.y
                    scaleX = zoom
                    scaleY = zoom
                    transformOrigin = TransformOrigin(0f, 0f)
                },
        ) {
            Wires(graph, place)
            graph.nodes.forEach { node ->
                NodeCard(
                    node,
                    at = place(node),
                    tap = if (editable) tap else null,
                    onDrag = { delta -> dragged[node.key] = place(node) + delta / density },
                    onTap = { if (editable && node.kind in listOf("statement", "event", "function")) menu = node },
                    onDropped = { place(node).let { onMove(node.key, it.x, it.y) } },
                )
            }
        }
        Row(Modifier.align(Alignment.BottomEnd).padding(10.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            IconButton(Sym.center_focus_strong, "Fit the graph", {
                val (fitPan, fitZoom) = fit(graph.nodes.map { place(it) to it }, wide, tall)
                pan = fitPan * density
                zoom = fitZoom
            }, Modifier.size(44.dp).clip(CircleShape).background(Forge.Raised), iconSize = 22.sp, color = Forge.Text)
        }
        if (handlers.size > 1) {
            Row(
                Modifier.align(Alignment.TopStart).fillMaxWidth().horizontalScroll(rememberScrollState()).padding(8.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                handlers.forEach { handler ->
                    val on = handler.key == current
                    val label = if (handler.kind == "values") "Values" else handler.title
                    Text(
                        label,
                        style = ForgeType.text(12.sp, FontWeight.SemiBold, if (on) Forge.MetalBright else Forge.TextMuted),
                        maxLines = 1,
                        modifier = Modifier
                            .clip(RoundedCornerShape(10.dp))
                            .background(if (on) Forge.MetalDeep else Forge.Raised)
                            .border(1.dp, if (on) Forge.MetalLine else Forge.Line, RoundedCornerShape(10.dp))
                            .clickable { shown = handler.key }
                            .padding(horizontal = 10.dp, vertical = 7.dp),
                    )
                }
            }
        }
        if (!editable) {
            Text(
                "Fix this script's errors as code to edit it here:\n" + script.problems.joinToString("\n"),
                style = ForgeType.text(12.sp, color = Forge.Danger),
                modifier = Modifier.align(Alignment.TopStart).padding(10.dp).background(Forge.Popover, RoundedCornerShape(10.dp)).padding(10.dp),
            )
        }
    }
    editing?.let { slot -> SlotDialog(slot) { value -> editing = null; value?.let { onEdit(slot, it) } } }
    menu?.let { node -> NodeMenu(node, script.palette, onDone = { menu = null }, onChange = onChange) }
}

/**
 * What can be done to a node: a statement deleted, moved past its neighbor,
 * or followed by a new one; a branch, loop or handler given a first step. A
 * new statement is chosen from the script's palette. Each is checked by the
 * host before the script changes, and a refusal says why.
 */
@Composable
private fun NodeMenu(
    node: DecayNode,
    palette: List<Pair<String, String>>,
    onDone: () -> Unit,
    onChange: (kind: String, span: Pair<Int, Int>, value: Any?) -> Unit,
) {
    // Where a chosen statement goes: after the node, or first in one of its branches.
    var adding by remember(node) { mutableStateOf<((String) -> Unit)?>(null) }
    val actions = buildList<Pair<String, () -> Unit>> {
        val statement = node.kind == "statement"
        if (statement) {
            add("Add after…" to { adding = { id -> onChange("insertAfter", node.span, id) } })
        }
        val branches = if (statement) node.outputs.filter { it.isNotEmpty() && it != "Next" } else listOf("")
        branches.forEachIndexed { index, branch ->
            val where = if (branch.isEmpty()) "Add first step…" else "Add inside $branch…"
            add(where to { adding = { id -> onChange("insertInside", node.span, mapOf("template" to id, "branch" to index)) } })
        }
        if (statement) {
            add("Move up" to { onChange("moveUp", node.span, null); onDone() })
            add("Move down" to { onChange("moveDown", node.span, null); onDone() })
            add("Delete" to { onChange("delete", node.span, null); onDone() })
        }
    }
    val choosing = adding
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDone,
        containerColor = Forge.Popover,
        title = { Text(if (choosing != null) "Add a step" else node.title, style = ForgeType.text(16.sp, FontWeight.SemiBold)) },
        text = {
            Column(
                Modifier.heightIn(max = 420.dp).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                if (choosing != null) {
                    palette.forEach { (id, label) -> MenuRow(label) { choosing(id); onDone() } }
                } else {
                    actions.forEach { (label, act) -> MenuRow(label, danger = label == "Delete", onClick = act) }
                }
            }
        },
        confirmButton = {},
        dismissButton = { androidx.compose.material3.TextButton(onClick = onDone) { Text("Cancel", color = Forge.TextMuted) } },
    )
}

@Composable
private fun MenuRow(label: String, danger: Boolean = false, onClick: () -> Unit) {
    Text(
        label,
        style = ForgeType.text(15.sp, color = if (danger) Forge.Danger else Forge.Text),
        modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp)).clickable(onClick = onClick).padding(horizontal = 8.dp, vertical = 12.dp),
    )
}

/** The part of [graph] that belongs to the handler keyed [handler]: it, its statements and their values. */
private fun only(graph: DecayGraph, handler: String?): DecayGraph {
    if (handler == null) return graph
    val nodes = graph.nodes.filter { it.key == handler || it.key.startsWith("$handler/") || it.key.startsWith("$handler:") }
    val keys = nodes.mapTo(HashSet()) { it.key }
    return DecayGraph(nodes, graph.wires.filter { it.from in keys && it.to in keys }, graph.flow)
}

/** The pan (in points) and zoom that show every node. */
private fun fit(nodes: List<Pair<Offset, DecayNode>>, width: Float, height: Float, top: Float = TABS): Pair<Offset, Float> {
    if (nodes.isEmpty()) return Offset(24f, top) to 1f
    val left = nodes.minOf { it.first.x }
    val right = nodes.maxOf { it.first.x + NODE_WIDTH }
    val bottom = nodes.maxOf { it.first.y + nodeHeight(it.second) }
    val zoom = minOf((width - 32f) / (right - left), (height - top - 16f) / (bottom - nodesTop(nodes))).coerceIn(0.3f, 1.2f)
    return Offset(16f - left * zoom, top - nodesTop(nodes) * zoom) to zoom
}

private fun nodesTop(nodes: List<Pair<Offset, DecayNode>>) = nodes.minOf { it.first.y }

/** The smallest zoom a graph opens at: below it, node text is too small to read. */
private const val READABLE = 0.55f

/** Room left at the top of the canvas for the handler tabs, in points. */
private const val TABS = 56f

/** Where execution enters a step from above, top to bottom: over its header's arrow. */
private const val FLOW_IN = 16f

/** The shared layout's measures, in points. */
private const val NODE_WIDTH = 240f
private const val HEADER = 36f
private const val ROW = 32f

private fun nodeHeight(node: DecayNode) = HEADER + maxOf(node.inputs.size, node.outputs.size, 1) * ROW + 8f

private fun inPin(node: DecayNode, at: Offset, name: String): Offset {
    val index = node.inputs.indexOfFirst { it.name == name }.coerceAtLeast(0)
    return Offset(at.x, at.y + HEADER + index * ROW + ROW / 2)
}

private fun outPin(node: DecayNode, at: Offset, name: String): Offset =
    if (node.givesValue) {
        Offset(at.x + NODE_WIDTH, at.y + HEADER / 2)
    } else {
        val index = node.outputs.indexOf(name).coerceAtLeast(0)
        Offset(at.x + NODE_WIDTH, at.y + HEADER + index * ROW + ROW / 2)
    }

/** Every wire, as a curve: execution in white, values in blue. */
@Composable
private fun Wires(graph: DecayGraph, place: (DecayNode) -> Offset) {
    val byKey = remember(graph) { graph.nodes.associateBy { it.key } }
    // A point-sized canvas that draws past its edges, which nothing clips. One
    // sized to the graph cannot be laid out: a long script's graph is tens of
    // thousands of pixels across at a phone's density, past what layout can
    // measure, and sizing to it crashed the sheet.
    Canvas(Modifier.size(1.dp)) {
        graph.wires.forEach { wire ->
            val from = byKey[wire.from] ?: return@forEach
            val to = byKey[wire.to] ?: return@forEach
            val down = wire.flow && graph.flow == "down"
            // A step with one way on hands execution straight down from its foot.
            val straight = down && from.outputs.size == 1
            val start = (if (straight) Offset(place(from).x + FLOW_IN, place(from).y + nodeHeight(from)) else outPin(from, place(from), wire.out)) * density
            val end = when {
                // Top to bottom, execution comes in at the top of the next step.
                down -> Offset(place(to).x + FLOW_IN, place(to).y)
                wire.flow -> Offset(place(to).x, place(to).y + HEADER / 2)
                else -> inPin(to, place(to), wire.into)
            } * density
            val path = Path().apply {
                moveTo(start.x, start.y)
                if (straight) {
                    lineTo(end.x, end.y)
                } else if (down) {
                    // Out of a labelled output to the right, round, and down into the top.
                    val drop = maxOf(30f * density, (end.y - start.y) / 2)
                    cubicTo(start.x + 36f * density, start.y, end.x, end.y - drop, end.x, end.y)
                } else {
                    val bend = maxOf(40f * density, (end.x - start.x) / 2)
                    cubicTo(start.x + bend, start.y, end.x - bend, end.y, end.x, end.y)
                }
            }
            drawPath(path, if (wire.flow) Forge.Text.copy(alpha = 0.8f) else Forge.Info, style = Stroke(width = (if (wire.flow) 2.5f else 2f) * density))
        }
    }
}

@Composable
private fun NodeCard(
    node: DecayNode,
    at: Offset,
    tap: ((DecaySlot) -> Unit)?,
    onDrag: (Offset) -> Unit,
    onDropped: () -> Unit,
    onTap: () -> Unit,
) {
    val tapped by rememberUpdatedState(onTap)
    val drag by rememberUpdatedState(onDrag)
    val dropped by rememberUpdatedState(onDropped)
    val (head, text) = when (node.kind) {
        "event", "function" -> Forge.MetalDeep to Forge.MetalBright
        "value" -> Color(0xFF13262B) to Forge.Info
        "values" -> Forge.Raised to Forge.Metal
        else -> Forge.Raised to Forge.Text
    }
    node.note?.let { note ->
        Text(
            note.replace('\n', ' '),
            style = ForgeType.text(11.sp, color = Forge.TextFaint).copy(fontStyle = FontStyle.Italic),
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.offset(at.x.dp, (at.y - 34).dp).width(NODE_WIDTH.dp),
        )
    }
    val shape = RoundedCornerShape(12.dp)
    Column(
        Modifier
            .offset(at.x.dp, at.y.dp)
            .size(NODE_WIDTH.dp, nodeHeight(node).dp)
            .clip(shape)
            .background(Forge.Card)
            .border(1.dp, if (node.kind == "event") Forge.MetalLine else Forge.Line, shape),
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .height(HEADER.dp)
                .background(head)
                .pointerInput(node.key) { detectTapGestures { tapped() } }
                .pointerInput(node.key) {
                    detectDragGestures(onDragEnd = { dropped() }) { change, amount ->
                        change.consume()
                        drag(amount)
                    }
                }
                .semantics { contentDescription = "${node.title}, drag to move, tap for its menu" }
                .padding(horizontal = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            if (node.takesFlow) {
                FlowPin()
            } else {
                Symbol(
                    when (node.kind) {
                        "event" -> Sym.bolt
                        "function" -> Sym.conversion_path
                        "values" -> Sym.tune
                        else -> Sym.code
                    },
                    15.sp,
                    text,
                )
            }
            Text(node.title, style = ForgeType.text(13.sp, FontWeight.SemiBold, text), maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            if (node.givesValue) ValuePin()
        }
        repeat(maxOf(node.inputs.size, node.outputs.size)) { row ->
            Row(Modifier.fillMaxWidth().height(ROW.dp).padding(horizontal = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    node.inputs.getOrNull(row)?.let { pin ->
                        ValuePin(hollow = pin.value != null)
                        if (pin.name.isNotEmpty()) Text(pin.name, style = ForgeType.text(11.sp, color = Forge.TextFaint))
                        pin.value?.let { value ->
                            SlotChip(value, if (tap != null && value.editable) ({ tap(value) }) else null, Modifier.weight(1f, fill = false))
                        }
                    }
                }
                node.outputs.getOrNull(row)?.let { output ->
                    if (output.isNotEmpty()) Text(output, style = ForgeType.text(11.sp, color = Forge.TextMuted), modifier = Modifier.padding(end = 6.dp))
                    FlowPin()
                }
            }
        }
    }
}

/** An execution pin: a small arrowhead. */
@Composable
private fun FlowPin() {
    Symbol(Sym.play_arrow, 13.sp, Forge.Text)
}

/** A value pin: a dot, hollow where the value is written on the pin. */
@Composable
private fun ValuePin(hollow: Boolean = false) {
    Box(
        Modifier
            .size(9.dp)
            .clip(CircleShape)
            .background(if (hollow) Color.Transparent else Forge.Info)
            .border(1.5.dp, Forge.Info, CircleShape),
    )
}
