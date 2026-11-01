package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * The selection's properties, as the prototype's inspector shows them: a
 * header with the object's on/off switch, then a card per section of friendly
 * rows. Half height first; the handle takes it to full. While paused it edits
 * the running game, and says so.
 */
@Composable
fun BoxScope.InspectorSheet(sheet: EditorSheet.Inspector, onEvent: (EditorEvent) -> Unit) {
    val height = if (sheet.full) SheetHeight.Below(56.dp) else SheetHeight.Share(520.dp, 0.58f)
    SheetFrame(height, onHandle = { onEvent(EditorEvent.ToggleDetent) }) {
        val inspector = sheet.inspector
        if (sheet.live) {
            Text(
                "Live: changes apply to this run now. At Stop you choose which to keep.",
                style = ForgeType.text(13.sp).copy(lineHeight = 18.sp),
                modifier = Modifier
                    .padding(start = 16.dp, end = 16.dp, bottom = 10.dp)
                    .fillMaxWidth()
                    .background(Forge.MetalDeep, RoundedCornerShape(12.dp))
                    .border(1.dp, Forge.MetalLine, RoundedCornerShape(12.dp))
                    .padding(horizontal = 12.dp, vertical = 10.dp),
            )
        }
        Row(Modifier.padding(start = 16.dp, end = 6.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Box(Modifier.size(44.dp).background(Forge.Raised, RoundedCornerShape(12.dp)), contentAlignment = Alignment.Center) { Symbol(inspector.icon, 24.sp, Forge.Metal) }
            Column(Modifier.weight(1f)) {
                Text(inspector.name, style = ForgeType.text(17.sp, FontWeight.SemiBold))
                Text(inspector.detail, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(top = 2.dp))
            }
            Toggle(inspector.active, { onEvent(EditorEvent.ToggleActive) }, "${inspector.name} is on")
            IconButton(Sym.close, "Close", { onEvent(EditorEvent.CloseSheet) }, color = Forge.TextFaint)
        }
        SheetBody(horizontal = 12.dp, gap = 10.dp) {
            inspector.sections.forEach { section -> SectionCard(section, onEvent) }
            if (inspector.canAddComponent) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .height(52.dp)
                        .clip(RoundedCornerShape(16.dp))
                        .dashed(Forge.LineStrong, 16f)
                        .clickable(role = Role.Button) { onEvent(EditorEvent.AddComponent) },
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                ) {
                    Symbol(Sym.add, 20.sp, Forge.TextMuted)
                    Text("Add a component", style = ForgeType.text(14.sp, color = Forge.TextMuted))
                }
            }
        }
    }
}

@Composable
private fun SectionCard(section: SectionUi, onEvent: (EditorEvent) -> Unit) {
    Column(Modifier.fillMaxWidth().card(radius = 16.dp).padding(start = 14.dp, end = 14.dp, top = 4.dp, bottom = 10.dp)) {
        Row(Modifier.height(38.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Symbol(section.icon, 18.sp, Forge.TextMuted)
            Text(section.title, style = ForgeType.text(13.sp, FontWeight.SemiBold), modifier = Modifier.weight(1f))
            Text(section.note, style = ForgeType.text(11.sp, color = Forge.TextFaint))
        }
        section.rows.forEach { row ->
            val change = { change: RowChange -> onEvent(EditorEvent.InspectorRow(section.title, row.label, change)) }
            when (row) {
                is RowUi.Stepper -> StepperRow(row, change)
                is RowUi.Slider -> SliderRow(row, change)
                is RowUi.Toggle -> Row(
                    Modifier.fillMaxWidth().height(46.dp).clickable { change(RowChange.Switch(!row.on)) },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(row.label, style = ForgeType.text(13.sp, color = Forge.TextMuted), modifier = Modifier.weight(1f))
                    Toggle(row.on, { change(RowChange.Switch(it)) }, row.label)
                }
                is RowUi.Segment -> Row(Modifier.padding(vertical = 5.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(row.label, style = ForgeType.text(13.sp, color = Forge.TextMuted), modifier = Modifier.weight(1f))
                    Segmented(row.options, row.chosen, { change(RowChange.Choose(it)) })
                }
                is RowUi.Swatches -> Row(Modifier.padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(row.label, style = ForgeType.text(13.sp, color = Forge.TextMuted), modifier = Modifier.weight(1f))
                    row.colors.forEachIndexed { index, color -> Swatch(color, index == row.chosen) { change(RowChange.Choose(index)) } }
                }
                is RowUi.SpriteRef -> Row(Modifier.padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Box(
                        Modifier.size(56.dp).clip(RoundedCornerShape(12.dp)).background(Forge.Well).border(1.dp, Forge.Hairline, RoundedCornerShape(12.dp)),
                        contentAlignment = Alignment.BottomCenter,
                    ) {
                        when {
                            row.image != null -> ProjectSprite(row.image, Modifier.size(44.dp).padding(bottom = 4.dp))
                            row.sheet != null -> SpriteThumb(row.sheet, row.cell, 52.dp)
                            else -> Symbol(Sym.image, 24.sp, Forge.TextFaint, Modifier.padding(bottom = 14.dp))
                        }
                    }
                    Column(Modifier.weight(1f)) {
                        Text(row.file, style = ForgeType.text(14.sp, FontWeight.Medium).copy(fontFamily = ForgeType.Mono))
                        Text(row.detail, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(top = 2.dp))
                    }
                    SecondaryButton("Swap", { change(RowChange.Open) }, height = 40.dp)
                }
                is RowUi.Behaviour -> Row(
                    Modifier.fillMaxWidth().clickable { onEvent(EditorEvent.OpenBehaviour) }.padding(top = 8.dp, bottom = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(row.label, style = ForgeType.text(14.sp).copy(lineHeight = 20.sp), modifier = Modifier.weight(1f))
                    Symbol(Sym.chevron_right, 22.sp, Forge.Metal)
                }
                is RowUi.Facts -> Column(Modifier.padding(top = 6.dp, bottom = 4.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    row.items.forEach { (key, value) ->
                        Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                            Text(key, style = ForgeType.text(13.sp, color = Forge.TextFaint))
                            Text(value, style = ForgeType.text(12.sp, color = Forge.TextMuted).copy(fontFamily = ForgeType.Mono), textAlign = TextAlign.End, modifier = Modifier.weight(1f))
                        }
                    }
                }
                is RowUi.More -> Row(
                    Modifier.fillMaxWidth().height(40.dp).clickable { change(RowChange.Open) },
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Symbol(if (row.expanded) Sym.expand_less else Sym.expand_more, 18.sp, Forge.Metal)
                    Text(row.label, style = ForgeType.text(13.sp, color = Forge.Metal))
                }
            }
        }
    }
}

@Composable
private fun StepperRow(row: RowUi.Stepper, change: (RowChange) -> Unit) {
    Row(Modifier.padding(vertical = 5.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        if (row.axis.isNotEmpty()) {
            Box(Modifier.size(22.dp).background(row.axisColor.copy(alpha = 0.2f), RoundedCornerShape(6.dp)), contentAlignment = Alignment.Center) {
                Text(row.axis, style = ForgeType.text(12.sp, FontWeight.Bold, row.axisColor))
            }
        }
        Text(row.label, style = ForgeType.text(13.sp, color = Forge.TextMuted), modifier = Modifier.weight(1f))
        Row {
            StepButton(Sym.remove, "Less ${row.label}", RoundedCornerShape(topStart = 12.dp, bottomStart = 12.dp)) { change(RowChange.Minus) }
            Box(
                Modifier.size(60.dp, 40.dp).background(Forge.Well).border(1.dp, Forge.Line),
                contentAlignment = Alignment.Center,
            ) { Text(row.value, style = ForgeType.text(14.sp)) }
            StepButton(Sym.add, "More ${row.label}", RoundedCornerShape(topEnd = 12.dp, bottomEnd = 12.dp)) { change(RowChange.Plus) }
        }
    }
}

@Composable
private fun StepButton(icon: Sym, description: String, shape: RoundedCornerShape, onClick: () -> Unit) {
    Box(
        Modifier
            .size(44.dp, 40.dp)
            .clip(shape)
            .background(Forge.Raised)
            .border(1.dp, Forge.Line, shape)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) { Symbol(icon, 20.sp, Forge.Text) }
}

/** The prototype's slider: a 4 dp track, a metal fill and a 24 dp knob, dragged or tapped. */
@Composable
fun SliderRow(row: RowUi.Slider, change: (RowChange) -> Unit) {
    Column(Modifier.padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row {
            Text(row.label, style = ForgeType.text(13.sp, color = Forge.TextMuted), modifier = Modifier.weight(1f))
            Text(row.value, style = ForgeType.text(13.sp, FontWeight.Medium))
        }
        BoxWithConstraints(
            Modifier
                .fillMaxWidth()
                .height(36.dp)
                .semantics { contentDescription = "${row.label} ${row.value}" }
                .pointerInput(row.label) {
                    detectTapGestures {
                        change(RowChange.To((it.x / size.width).coerceIn(0f, 1f)))
                        change(RowChange.Settled)
                    }
                }
                .pointerInput(row.label) {
                    detectHorizontalDragGestures(
                        onDragEnd = { change(RowChange.Settled) },
                        onDragCancel = { change(RowChange.Settled) },
                    ) { pointer, _ -> change(RowChange.To((pointer.position.x / size.width).coerceIn(0f, 1f))) }
                },
        ) {
            val fraction = row.fraction.coerceIn(0f, 1f)
            Box(Modifier.offset(y = 16.dp).fillMaxWidth().height(4.dp).background(Forge.Line, RoundedCornerShape(2.dp)))
            Box(Modifier.offset(y = 16.dp).width(maxWidth * fraction).height(4.dp).background(Forge.Metal, RoundedCornerShape(2.dp)))
            Box(
                Modifier
                    .offset(x = maxWidth * fraction - 12.dp, y = 6.dp)
                    .size(24.dp)
                    .shadow(4.dp, RoundedCornerShape(12.dp))
                    .background(Forge.Text, RoundedCornerShape(12.dp)),
            )
        }
    }
}

@Composable
private fun Swatch(color: Color, chosen: Boolean, onClick: () -> Unit) {
    Box(
        Modifier
            .size(34.dp)
            .then(if (chosen) Modifier.border(1.dp, Forge.Metal, RoundedCornerShape(17.dp)).padding(1.dp) else Modifier)
            .clip(RoundedCornerShape(17.dp))
            .background(if (chosen) Forge.Text else Forge.Card)
            .padding(3.dp)
            .clip(RoundedCornerShape(15.dp))
            .background(color)
            .clickable(role = Role.RadioButton, onClick = onClick),
    )
}

