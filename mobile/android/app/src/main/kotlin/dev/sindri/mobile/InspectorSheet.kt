package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.util.Locale

/**
 * The selection's inspector: a sheet over the bottom of the canvas, never a
 * panel that takes the canvas's room, as in the prototype. Numbers step with
 * large buttons or take a typed value; switches toggle. Every change goes to the
 * host as one checked, undoable edit, and a value the component refuses comes
 * back as the error card's message.
 */
@Composable
fun InspectorSheet(
    inspector: Inspector,
    onSet: (type: String, field: String, value: Any) -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier
            .fillMaxWidth()
            .heightIn(max = 360.dp)
            .background(Forge.Panel, RoundedCornerShape(18.dp))
            .border(1.dp, Forge.Line, RoundedCornerShape(18.dp))
            .padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(inspector.name, color = Forge.Text, fontSize = 16.sp, fontWeight = FontWeight.SemiBold)
                inspector.position?.let { position ->
                    Text(
                        position.joinToString("  ") { format(it) },
                        color = Forge.TextFaint,
                        fontSize = 12.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                }
            }
            RoundButton("✕", "Close inspector", onClose)
        }
        Column(
            Modifier.verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            for (component in inspector.components) {
                ComponentSection(component, onSet)
            }
            if (inspector.components.all { it.fields.isEmpty() }) {
                Text("Nothing here to edit by number or switch yet.", color = Forge.TextFaint, fontSize = 13.sp)
            }
        }
    }
}

@Composable
private fun ComponentSection(component: Component, onSet: (String, String, Any) -> Unit) {
    if (component.fields.isEmpty()) return
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(component.display.uppercase(Locale.ROOT), color = Forge.Metal, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
        for (field in component.fields) {
            FieldRow(field) { value -> onSet(component.type, field.name, value) }
        }
    }
}

@Composable
private fun FieldRow(field: Field, onSet: (Any) -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 48.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(field.name, color = Forge.TextMuted, fontSize = 14.sp, modifier = Modifier.weight(1f))
        if (field.isSwitch) {
            Switch(
                checked = field.value as Boolean,
                onCheckedChange = { onSet(it) },
                colors = SwitchDefaults.colors(checkedTrackColor = Forge.Metal, checkedThumbColor = Forge.Ground),
            )
        } else {
            val current = (field.value as Number).toDouble()
            // Whole-number fields stay whole numbers, so a schema expecting an
            // integer is never handed 4.0.
            val send = { value: Double -> onSet(if (field.isWhole) Math.round(value) else rounded(value)) }
            RoundButton("−", "Decrease ${field.name}") { send(current - field.step) }
            NumberField(field.name, current) { send(it) }
            RoundButton("+", "Increase ${field.name}") { send(current + field.step) }
        }
    }
}

/** A number that can be typed over; the value is sent when the keyboard's Done is pressed. */
@Composable
private fun NumberField(name: String, value: Double, onDone: (Double) -> Unit) {
    var text by remember(value) { mutableStateOf(format(value)) }
    BasicTextField(
        value = text,
        onValueChange = { text = it },
        singleLine = true,
        textStyle = TextStyle(color = Forge.Text, fontSize = 15.sp, fontFamily = FontFamily.Monospace),
        cursorBrush = SolidColor(Forge.Metal),
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { text.toDoubleOrNull()?.let(onDone) }),
        modifier = Modifier
            .width(76.dp)
            .background(Forge.PanelRaised, RoundedCornerShape(8.dp))
            .border(1.dp, Forge.Line, RoundedCornerShape(8.dp))
            .padding(horizontal = 10.dp, vertical = 10.dp)
            .semantics { contentDescription = name },
    )
}

@Composable
fun RoundButton(label: String, description: String, onClick: () -> Unit) {
    Box(
        Modifier
            .size(44.dp)
            .background(Forge.PanelRaised, RoundedCornerShape(12.dp))
            .border(1.dp, Forge.Line, RoundedCornerShape(12.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) {
        Text(label, color = Forge.Text, fontSize = 18.sp)
    }
}

/** Steps land on tenths, not on 0.30000000000000004. */
private fun rounded(value: Double) = Math.round(value * 1000.0) / 1000.0

private fun format(value: Double): String =
    if (value == Math.rint(value) && kotlin.math.abs(value) < 1e9) value.toLong().toString()
    else String.format(Locale.ROOT, "%.3f", value).trimEnd('0').trimEnd('.')
