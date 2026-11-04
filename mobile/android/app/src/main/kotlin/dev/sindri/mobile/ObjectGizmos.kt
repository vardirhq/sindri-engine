package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Editor-only markers for objects that have no sprite to tap. */
@Composable
fun ObjectGizmos(gizmos: List<GizmoUi>, onEvent: (EditorEvent) -> Unit) {
    gizmos.forEach { gizmo ->
        key(gizmo.id) { ObjectGizmo(gizmo, onEvent) }
    }
}

@Composable
private fun ObjectGizmo(gizmo: GizmoUi, onEvent: (EditorEvent) -> Unit) {
    val events by rememberUpdatedState(onEvent)
    var placed by remember { mutableStateOf<LayoutCoordinates?>(null) }
    val icon = when {
        gizmo.selected -> Sym.open_with
        gizmo.kind == "light" -> Sym.lightbulb
        gizmo.kind == "camera" -> Sym.camera_alt
        else -> Sym.deployed_code
    }
    Box(
        Modifier.offset((gizmo.x - 18).dp, (gizmo.y - 18).dp).size(36.dp)
            .clip(CircleShape)
            .background(if (gizmo.selected) Forge.Metal else Forge.Glass)
            .border(1.5.dp, Forge.MetalBright, CircleShape)
            .onGloballyPositioned { placed = it }
            .semantics { contentDescription = "${gizmo.name} object marker, drag to move" }
            .pointerInput(gizmo.id) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    down.consume()
                    val at = { point: Offset -> placed?.localToRoot(point) ?: point }
                    at(down.position).let { events(EditorEvent.GizmoTaken(gizmo.id, it.x, it.y)) }
                    while (true) {
                        val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                        if (!change.pressed) break
                        if (change.positionChanged()) {
                            change.consume()
                            at(change.position).let { events(EditorEvent.HandleDragged(it.x, it.y)) }
                        }
                    }
                    events(EditorEvent.HandleReleased)
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Symbol(icon, 20.sp, if (gizmo.selected) Forge.OnMetal else Forge.MetalBright)
    }
}
