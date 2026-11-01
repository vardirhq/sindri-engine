package dev.sindri.mobile

import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke

/**
 * A one-pixel dashed outline with corners of [radius] dp, for the "add"
 * affordances the prototype draws dashed: Add a component, Then…, New rule.
 */
fun Modifier.dashed(color: Color, radius: Float): Modifier = drawBehind {
    val px = density
    drawRoundRect(
        color = color,
        cornerRadius = CornerRadius(radius * px, radius * px),
        style = Stroke(width = px, pathEffect = PathEffect.dashPathEffect(floatArrayOf(4 * px, 3 * px), 0f)),
    )
}
