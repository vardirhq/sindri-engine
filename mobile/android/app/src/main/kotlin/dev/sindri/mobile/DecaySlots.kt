package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Whether a slot is edited on its pin: a number, text or a switch. */
val DecaySlot.editable get() = this is DecaySlot.Number || this is DecaySlot.Text || this is DecaySlot.Bool

/**
 * What a pin holds, as a chip: a literal in bright metal, tapped to change it;
 * a name plainly; code in the code face. [onTap] is given only where tapping
 * edits.
 */
@Composable
fun SlotChip(slot: DecaySlot, onTap: (() -> Unit)?, modifier: Modifier = Modifier) {
    val (text, literal) = when (slot) {
        is DecaySlot.Number -> InspectorPresenter.format(slot.value) to true
        is DecaySlot.Text -> "“${slot.value}”" to true
        is DecaySlot.Bool -> slot.value.toString() to true
        DecaySlot.Null -> "nothing" to false
        is DecaySlot.Name -> slot.path to false
        is DecaySlot.Code -> slot.source to false
        is DecaySlot.Call -> slot.callee + "(…)" to false
        is DecaySlot.Op -> slot.op to false
        is DecaySlot.Unary -> slot.op to false
    }
    val shape = RoundedCornerShape(8.dp)
    Text(
        text,
        style = ForgeType.text(12.sp, FontWeight.Medium, if (literal) Forge.MetalBright else Forge.Text)
            .let { if (slot is DecaySlot.Code) it.copy(fontFamily = ForgeType.Mono, fontWeight = FontWeight.Normal, color = Forge.TextMuted) else it },
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = modifier
            .clip(shape)
            .background(if (literal) Forge.MetalDeep else Forge.Raised)
            .border(1.dp, if (literal) Forge.MetalLine else Forge.Line, shape)
            .let { if (onTap != null) it.clickable(role = Role.Button, onClick = onTap) else it }
            .padding(horizontal = 7.dp, vertical = 3.dp),
    )
}

/** Asks for a slot's new number or text; null when cancelled. */
@Composable
fun SlotDialog(slot: DecaySlot, onDone: (Any?) -> Unit) {
    val number = slot is DecaySlot.Number
    var text by remember(slot) {
        mutableStateOf(
            when (slot) {
                is DecaySlot.Number -> InspectorPresenter.format(slot.value)
                is DecaySlot.Text -> slot.value
                else -> ""
            },
        )
    }
    val parsed: Any? = if (number) text.trim().toDoubleOrNull()?.takeIf { it.isFinite() } else text
    AlertDialog(
        onDismissRequest = { onDone(null) },
        containerColor = Forge.Popover,
        title = { Text(if (number) "Number" else "Text", style = ForgeType.text(16.sp, FontWeight.SemiBold)) },
        text = {
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                singleLine = number,
                keyboardOptions = KeyboardOptions(keyboardType = if (number) KeyboardType.Decimal else KeyboardType.Text),
                textStyle = ForgeType.text(15.sp),
            )
        },
        confirmButton = { TextButton(onClick = { onDone(parsed) }, enabled = parsed != null) { Text("Done", color = Forge.MetalBright) } },
        dismissButton = { TextButton(onClick = { onDone(null) }) { Text("Cancel", color = Forge.TextMuted) } },
    )
}
