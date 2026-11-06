package dev.sindri.mobile

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.min

/** A focused view of the object's authored collider, in object-local units. */
@Composable
fun BoxScope.ColliderSheet(sheet: EditorSheet.Collider, onEvent: (EditorEvent) -> Unit) {
    val collider = sheet.inspector.component("sindri.physics2d.collider") ?: return
    val pieces = (collider.values["pieces"] as? List<*>)?.size ?: 1
    val selected = sheet.piece.coerceIn(0, (pieces - 1).coerceAtLeast(0))
    val prefix = if (collider.values["pieces"] is List<*>) "pieces.$selected." else ""
    fun number(path: String, fallback: Double) = collider.number(prefix + path) ?: fallback
    fun vector(path: String, fallback: List<Double>): List<Double> = (collider.value(prefix + path) as? List<*>)?.mapNotNull { (it as? Number)?.toDouble() }?.takeIf { it.size == 2 } ?: fallback
    val shape = collider.value(prefix + "shape.shape") as? String ?: "box"
    val offset = vector("offset", listOf(0.0, 0.0))
    val extents = vector("shape.half_extents", listOf(0.5, 0.5))
    val radius = number("shape.radius", 0.5)
    val halfHeight = number("shape.half_height", 0.5)
    val angle = number("rotation", 0.0)
    fun set(path: String, value: Any) = onEvent(EditorEvent.ColliderField(prefix + path, value))

    SheetFrame(SheetHeight.Below(0.dp)) {
        SheetHeader("Collider · ${sheet.inspector.name}", onClose = { onEvent(EditorEvent.OpenInspector) })
        SheetBody(horizontal = 14.dp, gap = 10.dp) {
            Box(Modifier.fillMaxWidth().height(220.dp).background(Forge.Well, RoundedCornerShape(12.dp)), contentAlignment = Alignment.Center) {
                sheet.image?.let { ProjectSprite(it, Modifier.size(55.dp)) }
                Canvas(Modifier.matchParentSize()) {
                    val unit = min(size.width, size.height) / 4f
                    val center = Offset(size.width / 2f + offset[0].toFloat() * unit, size.height / 2f - offset[1].toFloat() * unit)
                    rotate((-angle * 180.0 / Math.PI).toFloat(), center) {
                        when (shape) {
                            "circle" -> drawCircle(Forge.MetalBright, radius.toFloat() * unit, center, style = Stroke(width = 3f))
                            "capsule" -> {
                                val r = radius.toFloat() * unit
                                val h = (halfHeight + radius).toFloat() * unit
                                drawRoundRect(Forge.MetalBright, Offset(center.x - r, center.y - h), Size(2f * r, 2f * h), CornerRadius(r, r), style = Stroke(width = 3f))
                            }
                            else -> {
                                val x = extents[0].toFloat() * unit
                                val y = extents[1].toFloat() * unit
                                drawRect(Forge.MetalBright, Offset(center.x - x, center.y - y), Size(2f * x, 2f * y), style = Stroke(width = 3f))
                            }
                        }
                    }
                }
            }
            Text("Shape ${selected + 1} of $pieces · object-local coordinates", style = ForgeType.text(12.sp, color = Forge.TextMuted))
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                repeat(pieces) { index -> Chip("${index + 1}", selected == index, { onEvent(EditorEvent.ColliderPiece(index)) }) }
                Chip("+ Shape", false, { onEvent(EditorEvent.AddColliderPiece) })
                if (pieces > 1) Chip("Remove", false, { onEvent(EditorEvent.RemoveColliderPiece) })
            }
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                listOf("box", "circle", "capsule").forEach { kind ->
                    Chip(kind.replaceFirstChar { it.uppercase() }, shape == kind, {
                        val value = when (kind) { "circle" -> mapOf("shape" to kind, "radius" to 0.5); "capsule" -> mapOf("shape" to kind, "radius" to 0.5, "half_height" to 0.5); else -> mapOf("shape" to kind, "half_extents" to listOf(0.5, 0.5)) }
                        set("shape", value)
                    })
                }
            }
            ColliderNumber("Offset X", offset[0], 0.1) { set("offset", listOf(it, offset[1])) }
            ColliderNumber("Offset Y", offset[1], 0.1) { set("offset", listOf(offset[0], it)) }
            ColliderNumber("Rotation (radians)", angle, 0.1) { set("rotation", it) }
            if (shape == "box") {
                ColliderNumber("Half width", extents[0], 0.1, 0.01) { set("shape.half_extents", listOf(it, extents[1])) }
                ColliderNumber("Half height", extents[1], 0.1, 0.01) { set("shape.half_extents", listOf(extents[0], it)) }
            } else {
                ColliderNumber("Radius", radius, 0.1, 0.01) { set("shape.radius", it) }
                if (shape == "capsule") ColliderNumber("Half height", halfHeight, 0.1, 0.01) { set("shape.half_height", it) }
            }
            ColliderNumber("Friction", number("friction", 0.5), 0.1, 0.0) { set("friction", it) }
            ColliderNumber("Restitution", number("restitution", 0.0), 0.1, 0.0) { set("restitution", it) }
            Row(Modifier.fillMaxWidth().height(48.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
                Text("Sensor (trigger)", style = ForgeType.text(14.sp))
                Toggle(collider.value(prefix + "sensor") == true, { set("sensor", it) }, "Sensor")
            }
        }
    }
}

@Composable
private fun ColliderNumber(label: String, value: Double, step: Double, minimum: Double = -1e6, onChange: (Double) -> Unit) {
    Row(Modifier.fillMaxWidth().height(48.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(label, style = ForgeType.text(14.sp), modifier = Modifier.weight(1f))
        Text("−", style = ForgeType.text(22.sp), modifier = Modifier.clickable { onChange((value - step).coerceAtLeast(minimum)) }.padding(10.dp))
        Text(String.format(java.util.Locale.ROOT, "%.2f", value), style = ForgeType.text(14.sp))
        Text("+", style = ForgeType.text(22.sp), modifier = Modifier.clickable { onChange(value + step) }.padding(10.dp))
    }
}
