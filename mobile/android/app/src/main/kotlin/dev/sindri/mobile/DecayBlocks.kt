package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * A Decay script as blocks: Decay, but visual. Each handler is a "When …"
 * card in the style of the design's rule cards, each statement a row in it,
 * nested where the code nests; numbers, text and switches are chips that edit
 * the script in place. Comments read as comments, and code with no block of
 * its own yet reads as code, so the blocks show everything the file holds.
 */
@Composable
fun DecayScriptView(script: DecayScript, onEdit: (DecaySlot, Any) -> Unit) {
    var editing by remember { mutableStateOf<DecaySlot?>(null) }
    val edit: (DecaySlot) -> Unit = { slot ->
        when (slot) {
            is DecaySlot.Bool -> if (script.problems.isEmpty()) onEdit(slot, !slot.value)
            is DecaySlot.Number, is DecaySlot.Text -> if (script.problems.isEmpty()) editing = slot
            else -> Unit
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        if (script.problems.isNotEmpty()) {
            Note(Sym.info, "This script has errors to fix as code before its blocks can be edited:\n" + script.problems.joinToString("\n"), Forge.Danger)
        }
        script.containers.forEach { container ->
            if (container.fields.isNotEmpty()) FieldsCard(container, edit)
            container.handlers.forEach { handler -> HandlerCard(handler, edit) }
        }
    }
    editing?.let { slot -> SlotDialog(slot, onDone = { value -> editing = null; value?.let { onEdit(slot, it) } }) }
}

@Composable
private fun FieldsCard(container: DecayContainer, edit: (DecaySlot) -> Unit) {
    Card {
        Header("VALUES", container.name, Sym.tune)
        Column(Modifier.padding(start = 12.dp, end = 12.dp, top = 8.dp, bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            container.fields.forEach { field ->
                field.note?.let { Comment(it) }
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        field.name,
                        style = ForgeType.text(13.sp, if (field.exported) FontWeight.SemiBold else FontWeight.Normal, if (field.exported) Forge.Text else Forge.TextMuted),
                        modifier = Modifier.weight(1f),
                    )
                    field.value?.let { SlotChips(it, edit) }
                }
            }
        }
    }
}

@Composable
private fun HandlerCard(handler: DecayHandler, edit: (DecaySlot) -> Unit) {
    Card {
        handler.note?.let { Box(Modifier.padding(start = 12.dp, end = 12.dp, top = 10.dp)) { Comment(it) } }
        val label = if (handler.params.isEmpty()) handler.label else "${handler.label} (${handler.params.joinToString()})"
        Header(if (handler.event) "WHEN" else "TO", label, if (handler.event) Sym.bolt else Sym.conversion_path)
        Column(Modifier.padding(start = 10.dp, end = 10.dp, top = 8.dp, bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            if (handler.body.isEmpty()) Text("Nothing yet", style = ForgeType.text(13.sp, color = Forge.TextFaint))
            Blocks(handler.body, edit)
        }
    }
}

@Composable
private fun Blocks(blocks: List<DecayBlock>, edit: (DecaySlot) -> Unit) {
    blocks.forEach { block ->
        when (block) {
            is DecayBlock.Set -> Statement("SET") { SlotChips(block.target, edit); Word("to"); SlotChips(block.value, edit) }
            is DecayBlock.Change -> Statement(changeWord(block.op)) { SlotChips(block.target, edit); Word("by"); SlotChips(block.value, edit) }
            is DecayBlock.Let -> Statement(if (block.mutable) "VAR" else "LET") {
                Word(block.name, Forge.Text)
                block.value?.let { Word("="); SlotChips(it, edit) }
            }
            is DecayBlock.Do -> Statement("DO") { SlotChips(block.call, edit) }
            is DecayBlock.If -> {
                Statement("IF") { SlotChips(block.condition, edit) }
                Nested { Blocks(block.then, edit) }
                block.otherwise?.let { otherwise ->
                    val chained = otherwise.singleOrNull() as? DecayBlock.If
                    if (chained != null) {
                        Blocks(listOf(chained), edit)
                    } else {
                        Statement("ELSE") {}
                        Nested { Blocks(otherwise, edit) }
                    }
                }
            }
            is DecayBlock.While -> {
                Statement("WHILE") { SlotChips(block.condition, edit) }
                Nested { Blocks(block.body, edit) }
            }
            is DecayBlock.For -> {
                Statement("FOR") { Word(block.name, Forge.Text); Word("in"); SlotChips(block.items, edit) }
                Nested { Blocks(block.body, edit) }
            }
            is DecayBlock.Match -> {
                Statement("MATCH") { SlotChips(block.subject, edit) }
                Nested {
                    block.arms.forEach { arm ->
                        Statement("IS") { arm.patterns.forEach { Word(it, Forge.Text) } }
                        Nested { Blocks(arm.body, edit) }
                    }
                }
            }
            is DecayBlock.Return -> Statement("GIVE") { block.value?.let { SlotChips(it, edit) } ?: Word("nothing back") }
            DecayBlock.Break -> Statement("STOP") { Word("the loop") }
            DecayBlock.Continue -> Statement("NEXT") { Word("go round again") }
            is DecayBlock.Comment -> Comment(block.text)
            is DecayBlock.Code -> Statement("CODE") { CodeText(block.source) }
        }
    }
}

private fun changeWord(op: String) = when (op) {
    "+=" -> "ADD"
    "-=" -> "TAKE"
    "*=" -> "TIMES"
    "/=" -> "SPLIT"
    else -> "MOD"
}

/** One expression as chips: literals editable, names and calls readable. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SlotChips(slot: DecaySlot, edit: (DecaySlot) -> Unit) {
    when (slot) {
        is DecaySlot.Number -> Chip(InspectorPresenter.format(slot.value), Forge.MetalBright, Forge.MetalDeep, Forge.MetalLine) { edit(slot) }
        is DecaySlot.Text -> Chip("“${slot.value}”", Forge.MetalBright, Forge.MetalDeep, Forge.MetalLine) { edit(slot) }
        is DecaySlot.Bool -> Chip(if (slot.value) "true" else "false", Forge.MetalBright, Forge.MetalDeep, Forge.MetalLine) { edit(slot) }
        DecaySlot.Null -> Chip("nothing", Forge.TextMuted, Forge.Raised, Forge.Line)
        is DecaySlot.Name -> Chip(slot.path, Forge.Text, Forge.Raised, Forge.Line)
        is DecaySlot.Call -> FlowRow(
            Modifier.clip(RoundedCornerShape(10.dp)).background(Forge.Raised).border(1.dp, Forge.Line, RoundedCornerShape(10.dp)).padding(horizontal = 6.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Text(slot.callee, style = ForgeType.text(13.sp, FontWeight.SemiBold), modifier = Modifier.align(Alignment.CenterVertically))
            slot.args.forEach { SlotChips(it, edit) }
        }
        is DecaySlot.Op -> FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            SlotChips(slot.left, edit)
            Word(opWord(slot.op))
            SlotChips(slot.right, edit)
        }
        is DecaySlot.Unary -> FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Word(if (slot.op == "!") "not" else "minus")
            SlotChips(slot.operand, edit)
        }
        is DecaySlot.Code -> CodeText(slot.source)
    }
}

private fun opWord(op: String) = when (op) {
    "&&" -> "and"
    "||" -> "or"
    "==" -> "is"
    "!=" -> "is not"
    "??" -> "or else"
    else -> op
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Statement(lead: String, content: @Composable () -> Unit) {
    Row(verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(lead, style = ForgeType.text(10.5.sp, FontWeight.Bold, Forge.TextFaint), modifier = Modifier.width(42.dp).padding(top = 9.dp))
        FlowRow(
            Modifier
                .weight(1f)
                .heightIn(min = 36.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(Forge.Well)
                .border(1.dp, Forge.Line, RoundedCornerShape(12.dp))
                .padding(horizontal = 8.dp, vertical = 5.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) { content() }
    }
}

@Composable
private fun Nested(content: @Composable () -> Unit) {
    Row {
        Box(Modifier.width(20.dp))
        Column(
            Modifier.weight(1f).padding(start = 8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) { content() }
    }
}

@Composable
private fun Chip(text: String, color: Color, fill: Color, line: Color, onClick: (() -> Unit)? = null) {
    val shape = RoundedCornerShape(9.dp)
    Text(
        text,
        style = ForgeType.text(13.sp, FontWeight.Medium, color),
        modifier = Modifier
            .clip(shape)
            .background(fill)
            .border(1.dp, line, shape)
            .let { if (onClick != null) it.clickable(role = Role.Button, onClick = onClick) else it }
            .padding(horizontal = 9.dp, vertical = 5.dp),
    )
}

@Composable
private fun Word(text: String, color: Color = Forge.TextFaint) {
    Text(text, style = ForgeType.text(13.sp, color = color), modifier = Modifier.padding(vertical = 5.dp))
}

@Composable
private fun CodeText(source: String) {
    Text(source, style = ForgeType.text(12.sp, color = Forge.TextMuted).copy(fontFamily = ForgeType.Mono), modifier = Modifier.padding(vertical = 5.dp))
}

/**
 * A comment as a paragraph: lines a person wrapped to fit their editor join
 * up, and a blank comment line still starts a new paragraph.
 */
@Composable
private fun Comment(text: String) {
    val paragraphs = text.split("\n\n").joinToString("\n\n") { it.replace('\n', ' ') }
    Text(paragraphs, style = ForgeType.text(12.sp, color = Forge.TextFaint).copy(fontStyle = FontStyle.Italic, lineHeight = 16.sp))
}

@Composable
private fun Note(icon: Sym, text: String, color: Color) {
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(Forge.Raised).padding(10.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Symbol(icon, 18.sp, color)
        Text(text, style = ForgeType.text(12.sp, color = Forge.TextMuted).copy(lineHeight = 16.sp))
    }
}

@Composable
private fun Card(content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(18.dp)
    Column(Modifier.fillMaxWidth().clip(shape).background(Forge.Card).border(1.dp, Forge.Line, shape)) { content() }
}

@Composable
private fun Header(lead: String, label: String, icon: Sym) {
    Row(
        Modifier.fillMaxWidth().background(Forge.Raised).padding(start = 12.dp, end = 12.dp, top = 8.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(lead, style = ForgeType.text(11.sp, FontWeight.Bold, Forge.Metal), modifier = Modifier.width(48.dp), softWrap = false)
        Row(
            Modifier
                .weight(1f)
                .heightIn(min = 40.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(Forge.MetalDeep)
                .border(1.dp, Forge.MetalLine, RoundedCornerShape(12.dp))
                .padding(horizontal = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Symbol(icon, 18.sp, Forge.MetalBright)
            Text(label, style = ForgeType.text(14.sp, FontWeight.Medium))
        }
    }
}

/** Asks for a slot's new number or text. */
@Composable
private fun SlotDialog(slot: DecaySlot, onDone: (Any?) -> Unit) {
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
