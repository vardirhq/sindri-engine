package dev.sindri.mobile

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.requiredSize
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
) {
    val graph = script.graph
    val editable = script.problems.isEmpty()
    var pan by remember { mutableStateOf(Offset(24f, 24f)) }
    var zoom by remember { mutableStateOf(0.8f) }
    // The first view shows the whole graph, unless that would make it too small to read.
    var framed by remember { mutableStateOf(false) }
    // Where nodes are while a finger drags them, ahead of the host.
    val dragged = remember(graph) { mutableStateMapOf<String, Offset>() }
    var editing by remember { mutableStateOf<DecaySlot?>(null) }
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
            val (fitPan, fitZoom) = fit(graph.nodes.map { Offset(it.x, it.y) to it }, wide, tall)
            zoom = fitZoom.coerceAtLeast(0.55f)
            pan = if (fitZoom >= 0.55f) fitPan * density else Offset(16f, 16f) * density
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
        if (!editable) {
            Text(
                "Fix this script's errors as code to edit it here:\n" + script.problems.joinToString("\n"),
                style = ForgeType.text(12.sp, color = Forge.Danger),
                modifier = Modifier.align(Alignment.TopStart).padding(10.dp).background(Forge.Popover, RoundedCornerShape(10.dp)).padding(10.dp),
            )
        }
    }
    editing?.let { slot -> SlotDialog(slot) { value -> editing = null; value?.let { onEdit(slot, it) } } }
}

/** The pan (in points) and zoom that show every node. */
private fun fit(nodes: List<Pair<Offset, DecayNode>>, width: Float, height: Float): Pair<Offset, Float> {
    if (nodes.isEmpty()) return Offset(24f, 24f) to 1f
    val left = nodes.minOf { it.first.x }
    val top = nodes.minOf { it.first.y }
    val right = nodes.maxOf { it.first.x + NODE_WIDTH }
    val bottom = nodes.maxOf { it.first.y + nodeHeight(it.second) }
    val zoom = minOf((width - 32f) / (right - left), (height - 32f) / (bottom - top)).coerceIn(0.3f, 1.2f)
    return Offset(16f - left * zoom, 16f - top * zoom) to zoom
}

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
    val far = graph.nodes.maxOfOrNull { maxOf(it.x, it.y) }?.plus(2000f) ?: 2000f
    Canvas(Modifier.requiredSize(far.dp)) {
        graph.wires.forEach { wire ->
            val from = byKey[wire.from] ?: return@forEach
            val to = byKey[wire.to] ?: return@forEach
            val start = outPin(from, place(from), wire.out) * density
            val end = (if (wire.flow) Offset(place(to).x, place(to).y + HEADER / 2) else inPin(to, place(to), wire.into)) * density
            val bend = maxOf(40f * density, (end.x - start.x) / 2)
            val path = Path().apply {
                moveTo(start.x, start.y)
                cubicTo(start.x + bend, start.y, end.x - bend, end.y, end.x, end.y)
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
) {
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
                .pointerInput(node.key) {
                    detectDragGestures(onDragEnd = { dropped() }) { change, amount ->
                        change.consume()
                        drag(amount)
                    }
                }
                .semantics { contentDescription = "${node.title}, drag to move" }
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
