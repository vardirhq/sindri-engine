package dev.sindri.mobile

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.toSize
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

/**
 * Where the workspace's canvas puts the object: [unit] pixels to one of its
 * units, with [middle] of the object at the canvas's middle.
 */
private data class Framing(val unit: Float, val middle: Vec2, val size: Size) {
    fun screen(point: Vec2) = Offset(
        size.width / 2 + ((point.x - middle.x) * unit).toFloat(),
        size.height / 2 - ((point.y - middle.y) * unit).toFloat(),
    )

    fun objectAt(offset: Offset) = Vec2(
        middle.x + (offset.x - size.width / 2) / unit,
        middle.y - (offset.y - size.height / 2) / unit,
    )

    companion object {
        fun of(parts: List<ObjectPart>, size: Size): Framing {
            val (low, high) = ObjectParts.bounds(parts)
            val span = Vec2(max(high.x - low.x, 1.0), max(high.y - low.y, 1.0))
            val unit = (min(size.width / span.x, size.height / span.y) * 0.8).toFloat()
            return Framing(unit.coerceAtLeast(1f), Vec2((low.x + high.x) / 2, (low.y + high.y) / 2), size)
        }
    }
}

/** What a drag in the workspace is doing. */
private sealed interface PartDrag {
    data class Moving(val part: ObjectPart, val from: Vec2) : PartDrag
    data class Resizing(val part: ObjectPart) : PartDrag
}

/**
 * The object workspace: the selected object on its own, in its own units, with
 * every part it can place drawn and draggable. Tap a part, or its chip, to
 * choose it; drag a part to move it and its handle to size it; its fields are
 * below. Each drag is one undo step, saved when the finger lifts.
 */
@Composable
fun BoxScope.ObjectWorkspace(sheet: EditorSheet.Parts, onEvent: (EditorEvent) -> Unit) {
    val parts = ObjectParts.of(sheet.inspector)
    val chosen = parts.firstOrNull { it.key == sheet.part } ?: parts.firstOrNull()
    SheetFrame(SheetHeight.Below(0.dp)) {
        val subtitle = sheet.prefab?.let { "${it.substringAfterLast('/')} · every instance follows" }
            ?: "${sheet.inspector.name} · in the object's own units"
        SheetHeader(if (sheet.prefab != null) "Prefab parts" else "Parts", subtitle, onClose = { onEvent(EditorEvent.OpenInspector) })
        PartsCanvas(parts, chosen, sheet.image, onEvent)
        Row(
            Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 14.dp, vertical = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            parts.forEach { part -> Chip(part.label, part.key == chosen?.key, { onEvent(EditorEvent.PartChosen(part.key)) }) }
        }
        SheetBody(horizontal = 14.dp, gap = 4.dp) {
            if (chosen == null) {
                Text("This object has nothing to place yet. Add art, a light or a collider from the inspector.", style = ForgeType.text(14.sp, color = Forge.TextMuted))
            } else {
                PartFields(chosen, sheet.inspector, parts, onEvent)
            }
        }
    }
}

@Composable
private fun PartsCanvas(parts: List<ObjectPart>, chosen: ObjectPart?, image: ProjectImage?, onEvent: (EditorEvent) -> Unit) {
    val bitmap = image?.let { rememberProjectBitmap(it.path) }
    var frozen by remember { mutableStateOf<Framing?>(null) }
    var size by remember { mutableStateOf(Size.Zero) }
    val currentParts by rememberUpdatedState(parts)
    val currentChosen by rememberUpdatedState(chosen)
    val events by rememberUpdatedState(onEvent)
    fun framing() = frozen ?: Framing.of(currentParts, size)
    val slopPixels = 22f

    Box(
        Modifier
            .padding(horizontal = 14.dp)
            .fillMaxWidth()
            .height(300.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(Forge.Well)
            .onSizeChanged { size = it.toSize() }
            .semantics { contentDescription = "The object's parts. ${chosen?.label ?: "Nothing"} is chosen." }
            .pointerInput(Unit) {
                detectTapGestures { at ->
                    val framing = framing()
                    val point = framing.objectAt(at)
                    ObjectParts.at(currentParts, point, ObjectParts.slop(slopPixels, framing.unit))
                        ?.let { events(EditorEvent.PartChosen(it.key)) }
                }
            }
            .pointerInput(Unit) {
                var drag: PartDrag? = null
                detectDragGestures(
                    onDragStart = { at ->
                        val framing = framing()
                        frozen = framing
                        val point = framing.objectAt(at)
                        val slop = ObjectParts.slop(slopPixels, framing.unit)
                        val selected = currentChosen
                        drag = when {
                            selected != null && ObjectParts.onHandle(selected, point, slop) -> PartDrag.Resizing(selected)
                            else -> ObjectParts.at(currentParts, point, slop)?.let { part ->
                                events(EditorEvent.PartChosen(part.key))
                                PartDrag.Moving(part, point)
                            }
                        }
                    },
                    onDrag = { change, _ ->
                        val framing = frozen ?: return@detectDragGestures
                        val point = framing.objectAt(change.position)
                        val edit = when (val now = drag) {
                            is PartDrag.Moving -> now.part.moved(now.part.offset, point - now.from)
                            is PartDrag.Resizing -> now.part.resized(point)
                            null -> null
                        }
                        if (edit != null) {
                            change.consume()
                            events(EditorEvent.PartDragged(edit))
                        }
                    },
                    onDragEnd = {
                        if (drag != null) events(EditorEvent.PartSettled)
                        drag = null
                        frozen = null
                    },
                    onDragCancel = {
                        if (drag != null) events(EditorEvent.PartSettled)
                        drag = null
                        frozen = null
                    },
                )
            },
    ) {
        Canvas(Modifier.matchParentSize()) {
            val framing = frozen ?: Framing.of(parts, this.size)
            grid(framing)
            parts.firstOrNull { it.type == ObjectParts.SPRITE }?.let { art ->
                if (bitmap != null) picture(bitmap, image, art, framing)
            }
            parts.forEach { part -> outline(part, framing, part.key == chosen?.key) }
            chosen?.handleAt()?.let { handle ->
                val at = framing.screen(handle)
                drawCircle(Forge.Well, 11.dp.toPx(), at)
                drawCircle(Forge.MetalBright, 8.dp.toPx(), at)
            }
        }
    }
}

private fun DrawScope.grid(framing: Framing) {
    if (framing.unit >= 12f) {
        val low = framing.objectAt(Offset.Zero)
        val high = framing.objectAt(Offset(size.width, size.height))
        var x = kotlin.math.floor(low.x)
        while (x <= high.x) {
            val at = framing.screen(Vec2(x, 0.0)).x
            drawLine(Forge.Hairline, Offset(at, 0f), Offset(at, size.height))
            x += 1.0
        }
        var y = kotlin.math.floor(high.y)
        while (y <= low.y) {
            val at = framing.screen(Vec2(0.0, y)).y
            drawLine(Forge.Hairline, Offset(0f, at), Offset(size.width, at))
            y += 1.0
        }
    }
    val origin = framing.screen(Vec2(0.0, 0.0))
    val arm = 10.dp.toPx()
    drawLine(Forge.TextFaint, origin - Offset(arm, 0f), origin + Offset(arm, 0f), 2f)
    drawLine(Forge.TextFaint, origin - Offset(0f, arm), origin + Offset(0f, arm), 2f)
}

/** The art, stretched over its quad as the renderer draws it, mirrored where its size is negative. */
private fun DrawScope.picture(bitmap: ImageBitmap, image: ProjectImage, art: ObjectPart, framing: Framing) {
    val box = art.outline as? PartOutline.Box ?: return
    val size = (art.handle as? PartHandle.Corner)?.size ?: return
    val (left, top, width, height) = image.rect
    val center = framing.screen(art.center)
    val across = (box.half.x * 2 * framing.unit).toFloat()
    val tall = (box.half.y * 2 * framing.unit).toFloat()
    if (across < 1f || tall < 1f) return
    scale(if (size.x < 0) -1f else 1f, if (size.y < 0) -1f else 1f, center) {
        drawImage(
            bitmap,
            srcOffset = IntOffset((left * bitmap.width).roundToInt(), (top * bitmap.height).roundToInt()),
            srcSize = IntSize((width * bitmap.width).roundToInt().coerceAtLeast(1), (height * bitmap.height).roundToInt().coerceAtLeast(1)),
            dstOffset = IntOffset((center.x - across / 2).roundToInt(), (center.y - tall / 2).roundToInt()),
            dstSize = IntSize(across.roundToInt(), tall.roundToInt()),
            filterQuality = FilterQuality.None,
        )
    }
}

private fun colorOf(part: ObjectPart, chosen: Boolean): Color = when {
    chosen -> Forge.MetalBright
    part.type == ObjectParts.COLLIDER -> Forge.Success
    part.type == ObjectParts.LIGHT -> Forge.Metal
    part.type == ObjectParts.SPRITE -> Forge.LineStrong
    else -> Forge.TextMuted
}

private fun DrawScope.outline(part: ObjectPart, framing: Framing, chosen: Boolean) {
    val color = colorOf(part, chosen)
    val stroke = Stroke(width = if (chosen) 3.dp.toPx() else 1.5.dp.toPx())
    val center = framing.screen(part.center)
    val unit = framing.unit
    fun degrees(radians: Double) = (-radians * 180.0 / Math.PI).toFloat()
    when (val outline = part.outline) {
        is PartOutline.Box -> rotate(degrees(outline.rotation), center) {
            val half = Offset((outline.half.x * unit).toFloat(), (outline.half.y * unit).toFloat())
            drawRect(color, center - half, Size(half.x * 2, half.y * 2), style = stroke)
        }
        is PartOutline.Circle -> drawCircle(color, max((outline.radius * unit).toFloat(), 3.dp.toPx()), center, style = stroke)
        is PartOutline.Capsule -> rotate(degrees(outline.rotation), center) {
            val r = (outline.radius * unit).toFloat()
            val h = ((outline.halfHeight + outline.radius) * unit).toFloat()
            drawRoundRect(color, Offset(center.x - r, center.y - h), Size(2 * r, 2 * h), CornerRadius(r, r), style = stroke)
        }
        is PartOutline.Ellipse -> {
            val half = Offset((outline.half.x * unit).toFloat(), (outline.half.y * unit).toFloat())
            drawOval(color, center - half, Size(half.x * 2, half.y * 2), style = stroke)
        }
        is PartOutline.Reach -> {
            val dashes = PathEffect.dashPathEffect(floatArrayOf(10f, 8f))
            drawCircle(color.copy(alpha = 0.6f), (outline.radius * unit).toFloat(), center, style = Stroke(stroke.width, pathEffect = dashes))
            drawCircle(color, 6.dp.toPx(), center)
        }
    }
}

/** The chosen part's numbers: where it sits, and how big it is. */
@Composable
private fun PartFields(part: ObjectPart, inspector: Inspector, parts: List<ObjectPart>, onEvent: (EditorEvent) -> Unit) {
    fun write(path: String, value: Any) = onEvent(EditorEvent.PartField(PartEdit(part.type, path, value)))
    PartNumber("Offset across", part.offset.x, 0.05) { write(part.offsetPath, listOf(it, part.offset.y)) }
    PartNumber("Offset up", part.offset.y, 0.05) { write(part.offsetPath, listOf(part.offset.x, it)) }
    when (val handle = part.handle) {
        is PartHandle.Corner -> {
            val (width, height) = when (part.type) {
                ObjectParts.SPRITE -> "Width" to "Height"
                else -> "Shape width" to "Shape height"
            }
            fun keepSign(value: Double, was: Double) = abs(value).coerceAtLeast(ObjectPart.MINIMUM) * (if (was < 0) -1.0 else 1.0)
            PartNumber(width, abs(handle.size.x), 0.05, ObjectPart.MINIMUM) { write(handle.path, listOf(keepSign(it, handle.size.x), handle.size.y)) }
            PartNumber(height, abs(handle.size.y), 0.05, ObjectPart.MINIMUM) { write(handle.path, listOf(handle.size.x, keepSign(it, handle.size.y))) }
        }
        is PartHandle.Ring -> (part.outline as? PartOutline.Reach)?.let { reach ->
            PartNumber("Reach", reach.radius, 0.25, ObjectPart.MINIMUM) { write(handle.path, it) }
        }
        null -> Unit
    }
    ObjectParts.colliderPiece(part.key)?.let { index ->
        inspector.component(ObjectParts.COLLIDER)?.let { collider ->
            ColliderPieceControls(collider, index, parts.count { it.type == ObjectParts.COLLIDER }, onEvent)
        }
    }
    if (part.type == ObjectParts.PARTICLES && part.handle == null) {
        Text("Points and circles are sized in the inspector.", style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(top = 6.dp))
    }
}

/** One collider shape's kind, turn, size and surface, as the collider workspace edited them. */
@Composable
private fun ColliderPieceControls(collider: Component, index: Int, count: Int, onEvent: (EditorEvent) -> Unit) {
    val prefix = "pieces.$index."
    fun number(path: String, fallback: Double) = collider.number(prefix + path) ?: fallback
    fun vector(path: String, fallback: List<Double>): List<Double> =
        (collider.value(prefix + path) as? List<*>)?.mapNotNull { (it as? Number)?.toDouble() }?.takeIf { it.size == 2 } ?: fallback
    fun set(path: String, value: Any) = onEvent(EditorEvent.ColliderField(prefix + path, value))
    val shape = collider.value(prefix + "shape.shape") as? String ?: "box"
    val extents = vector("shape.half_extents", listOf(0.5, 0.5))
    Row(Modifier.padding(vertical = 6.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        listOf("box", "circle", "capsule").forEach { kind ->
            Chip(kind.replaceFirstChar { it.uppercase() }, shape == kind, {
                val value = when (kind) {
                    "circle" -> mapOf("shape" to kind, "radius" to 0.5)
                    "capsule" -> mapOf("shape" to kind, "radius" to 0.5, "half_height" to 0.5)
                    else -> mapOf("shape" to kind, "half_extents" to listOf(0.5, 0.5))
                }
                set("shape", value)
            })
        }
    }
    PartNumber("Turn (radians)", number("rotation", 0.0), 0.1) { set("rotation", it) }
    if (shape == "box") {
        PartNumber("Half width", extents[0], 0.05, 0.01) { set("shape.half_extents", listOf(it, extents[1])) }
        PartNumber("Half height", extents[1], 0.05, 0.01) { set("shape.half_extents", listOf(extents[0], it)) }
    } else {
        PartNumber("Radius", number("shape.radius", 0.5), 0.05, 0.01) { set("shape.radius", it) }
        if (shape == "capsule") PartNumber("Half height", number("shape.half_height", 0.5), 0.05, 0.01) { set("shape.half_height", it) }
    }
    PartNumber("Friction", number("friction", 0.5), 0.1, 0.0) { set("friction", it) }
    PartNumber("Bounce", number("restitution", 0.0), 0.1, 0.0) { set("restitution", it) }
    Row(Modifier.fillMaxWidth().height(48.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
        Text("Sensor (trigger)", style = ForgeType.text(14.sp))
        Toggle(collider.value(prefix + "sensor") == true, { set("sensor", it) }, "Sensor")
    }
    Row(Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Chip("+ Collider shape", false, { onEvent(EditorEvent.AddColliderPiece) })
        if (count > 1) Chip("Remove this shape", false, { onEvent(EditorEvent.RemoveColliderPiece) })
    }
}

@Composable
private fun PartNumber(label: String, value: Double, step: Double, minimum: Double = -1e6, onChange: (Double) -> Unit) {
    fun rounded(next: Double) = (Math.round(next / step) * step).coerceAtLeast(minimum)
    Row(Modifier.fillMaxWidth().height(48.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(label, style = ForgeType.text(14.sp), modifier = Modifier.weight(1f))
        Text("−", style = ForgeType.text(22.sp), modifier = Modifier.clickable(onClickLabel = "Less $label") { onChange(rounded(value - step)) }.padding(10.dp))
        Text(String.format(java.util.Locale.ROOT, "%.2f", value), style = ForgeType.text(14.sp))
        Text("+", style = ForgeType.text(22.sp), modifier = Modifier.clickable(onClickLabel = "More $label") { onChange(rounded(value + step)) }.padding(10.dp))
    }
}
