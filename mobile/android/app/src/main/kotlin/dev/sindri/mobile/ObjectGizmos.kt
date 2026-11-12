package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Editor-only markers for objects that have nothing drawn to tap.
 *
 * They only draw. Touches pass through to the canvas, where the engine decides
 * what a finger hit: the drawn art first, then the nearest marker. So a marker
 * never covers a sprite, and like everything on the canvas it is tapped to be
 * selected before a drag moves it.
 */
@Composable
fun ObjectGizmos(gizmos: List<GizmoUi>) {
    gizmos.forEach { gizmo ->
        key(gizmo.id) { ObjectGizmo(gizmo) }
    }
}

@Composable
private fun ObjectGizmo(gizmo: GizmoUi) {
    val icon = when {
        gizmo.selected -> Sym.open_with
        gizmo.kind == "light" -> Sym.lightbulb
        gizmo.kind == "camera" -> Sym.camera_alt
        else -> Sym.deployed_code
    }
    val size = if (gizmo.selected) 32 else 24
    Box(
        Modifier.offset((gizmo.x - size / 2).dp, (gizmo.y - size / 2).dp).size(size.dp)
            .clip(CircleShape)
            .background(if (gizmo.selected) Forge.Metal else Forge.Glass)
            .border(if (gizmo.selected) 1.5.dp else 1.dp, if (gizmo.selected) Forge.MetalBright else Forge.Metal, CircleShape)
            .semantics {
                contentDescription = if (gizmo.selected) "${gizmo.name}, selected, drag to move" else "${gizmo.name} marker, tap to select"
            },
        contentAlignment = Alignment.Center,
    ) {
        Symbol(icon, if (gizmo.selected) 18.sp else 14.sp, if (gizmo.selected) Forge.OnMetal else Forge.MetalBright)
    }
}
